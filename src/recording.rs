use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

const MAX_RECORDING_BYTES: u64 = 128 * 1024 * 1024;
const MAX_RECORDING_SECONDS: u64 = 10 * 60;

/// A bounded PCM16 WAV writer that refuses to replace an existing path.
pub(crate) struct Pcm16WavRecorder {
    path: PathBuf,
    writer: Option<hound::WavWriter<File>>,
    channels: u16,
    sample_rate: u32,
    samples_written: u64,
    maximum_samples: u64,
}

impl Pcm16WavRecorder {
    pub(crate) fn create(path: &Path, channels: u16, sample_rate: u32) -> Result<Self, String> {
        if !matches!(channels, 1 | 2) || !(8_000..=96_000).contains(&sample_rate) {
            return Err("Recording requires mono or stereo input at 8–96 kHz".into());
        }
        let bytes_per_frame = u64::from(channels) * 2;
        let max_frames_by_bytes = MAX_RECORDING_BYTES.saturating_sub(128) / bytes_per_frame;
        let maximum_samples = max_frames_by_bytes
            .min(u64::from(sample_rate) * MAX_RECORDING_SECONDS)
            .saturating_mul(u64::from(channels));
        Self::create_with_sample_limit(path, channels, sample_rate, maximum_samples)
    }

    fn create_with_sample_limit(
        path: &Path,
        channels: u16,
        sample_rate: u32,
        maximum_samples: u64,
    ) -> Result<Self, String> {
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
        {
            return Err("Recording output must use the .wav extension".into());
        }
        if !matches!(channels, 1 | 2) || !(8_000..=96_000).contains(&sample_rate) {
            return Err("Recording requires mono or stereo input at 8–96 kHz".into());
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "Refusing to overwrite existing recording: {}",
                        path.display()
                    )
                } else {
                    format!("Could not create recording: {error}")
                }
            })?;
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let writer = match hound::WavWriter::new(file, spec) {
            Ok(writer) => writer,
            Err(error) => {
                let _ = fs::remove_file(path);
                return Err(format!("Could not initialize WAV recording: {error}"));
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            writer: Some(writer),
            channels,
            sample_rate,
            samples_written: 0,
            maximum_samples,
        })
    }

    pub(crate) fn append(&mut self, samples: &[i16]) -> Result<(), String> {
        if !samples.len().is_multiple_of(usize::from(self.channels)) {
            return Err("Input recording buffer ended in a partial audio frame".into());
        }
        let next_count = self
            .samples_written
            .checked_add(samples.len() as u64)
            .ok_or_else(|| "Recording sample count overflow".to_owned())?;
        if next_count > self.maximum_samples {
            return Err("Recording reached the 10-minute or 128 MiB safety limit".into());
        }
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| "Recording writer is already closed".to_owned())?;
        for sample in samples {
            writer
                .write_sample(*sample)
                .map_err(|error| format!("Could not write recording: {error}"))?;
        }
        self.samples_written = next_count;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<(PathBuf, u64, u32), String> {
        let writer = self
            .writer
            .take()
            .ok_or_else(|| "Recording writer is already closed".to_owned())?;
        writer
            .finalize()
            .map_err(|error| format!("Could not finalize WAV recording: {error}"))?;
        Ok((
            self.path.clone(),
            self.samples_written / u64::from(self.channels),
            self.sample_rate,
        ))
    }

    pub(crate) fn abort(mut self) -> Result<(), String> {
        drop(self.writer.take());
        fs::remove_file(&self.path)
            .map_err(|error| format!("Could not remove cancelled recording: {error}"))
    }

    #[cfg(feature = "audio-playback")]
    pub(crate) fn duration_ms(&self) -> u64 {
        (self.samples_written / u64::from(self.channels)).saturating_mul(1000)
            / u64::from(self.sample_rate)
    }
}

fn append_capture_blocks(
    writer: &mut Pcm16WavRecorder,
    blocks: impl IntoIterator<Item = Vec<i16>>,
) -> Option<String> {
    for block in blocks {
        if let Err(error) = writer.append(&block) {
            return Some(error);
        }
    }
    None
}

pub(crate) struct FinalizedRecording {
    pub(crate) path: PathBuf,
    pub(crate) frames: u64,
    pub(crate) sample_rate: u32,
    pub(crate) partial_reason: Option<String>,
}

fn finalize_pending_capture(
    mut writer: Pcm16WavRecorder,
    blocks: impl IntoIterator<Item = Vec<i16>>,
    mut partial_reason: Option<String>,
) -> Result<FinalizedRecording, String> {
    if let Some(write_error) = append_capture_blocks(&mut writer, blocks) {
        add_partial_reason(&mut partial_reason, &write_error);
    }
    finish_recording_writer(writer, partial_reason)
}

fn finish_recording_writer(
    writer: Pcm16WavRecorder,
    partial_reason: Option<String>,
) -> Result<FinalizedRecording, String> {
    let (path, frames, sample_rate) = writer.finish()?;
    Ok(FinalizedRecording {
        path,
        frames,
        sample_rate,
        partial_reason,
    })
}

#[cfg(feature = "audio-playback")]
mod capture {
    use super::{
        add_partial_reason, finalize_pending_capture, FinalizedRecording, Pcm16WavRecorder,
    };
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::{
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
            Arc,
        },
    };

    const CAPTURE_BLOCK_QUEUE: usize = 16;
    const MAX_CALLBACK_SAMPLES: usize = 32_768;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct InputDevice {
        pub(crate) index: usize,
        pub(crate) label: String,
    }

    pub(crate) struct CaptureSession {
        stream: Option<cpal::Stream>,
        samples: Receiver<Vec<i16>>,
        writer: Option<Pcm16WavRecorder>,
        overflowed: Arc<AtomicBool>,
        stream_failed: Arc<AtomicBool>,
        path: String,
    }

    pub(crate) fn enumerate_inputs() -> Result<Vec<InputDevice>, String> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|error| format!("Could not enumerate audio input devices: {error}"))?;
        Ok(devices
            .enumerate()
            .map(|(index, device)| {
                let name = device
                    .name()
                    .unwrap_or_else(|_| "Unnamed input device".into());
                InputDevice {
                    index,
                    label: format!("{index}: {name}"),
                }
            })
            .collect())
    }

    pub(crate) fn start_capture(
        path: &Path,
        selected_device: Option<usize>,
    ) -> Result<CaptureSession, String> {
        let host = cpal::default_host();
        let device = if let Some(index) = selected_device {
            host.input_devices()
                .map_err(|error| format!("Could not enumerate input devices: {error}"))?
                .nth(index)
                .ok_or_else(|| "Selected audio input is no longer available".to_owned())?
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default audio input device is available".to_owned())?
        };
        let supported = device
            .default_input_config()
            .map_err(|error| format!("Could not read input device configuration: {error}"))?;
        let channels = supported.channels();
        let sample_rate = supported.sample_rate().0;
        if !matches!(channels, 1 | 2) || !(8_000..=96_000).contains(&sample_rate) {
            return Err("Selected device default must be mono/stereo at 8–96 kHz".into());
        }
        let config: cpal::StreamConfig = supported.clone().into();
        let (sender, samples) = mpsc::sync_channel(CAPTURE_BLOCK_QUEUE);
        let overflowed = Arc::new(AtomicBool::new(false));
        let stream_failed = Arc::new(AtomicBool::new(false));
        let writer = Pcm16WavRecorder::create(path, channels, sample_rate)?;
        let overflow_callback = Arc::clone(&overflowed);
        let failure_callback = Arc::clone(&stream_failed);
        let stream_result = match supported.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config,
                sender,
                overflow_callback,
                failure_callback,
            ),
            cpal::SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config,
                sender,
                overflow_callback,
                failure_callback,
            ),
            cpal::SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                sender,
                overflow_callback,
                failure_callback,
            ),
            format => Err(format!("Unsupported input sample format: {format:?}")),
        };
        let stream = match stream_result {
            Ok(stream) => stream,
            Err(error) => {
                return match writer.abort() {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(format!("{error}; {cleanup}")),
                };
            }
        };
        if let Err(error) = stream.play() {
            drop(stream);
            return match writer.abort() {
                Ok(()) => Err(format!("Could not start input stream: {error}")),
                Err(cleanup) => Err(format!("Could not start input stream: {error}; {cleanup}")),
            };
        }
        Ok(CaptureSession {
            stream: Some(stream),
            samples,
            writer: Some(writer),
            overflowed,
            stream_failed,
            path: path.display().to_string(),
        })
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        sender: SyncSender<Vec<i16>>,
        overflowed: Arc<AtomicBool>,
        stream_failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, String>
    where
        T: cpal::Sample + cpal::SizedSample + Copy,
        i16: cpal::FromSample<T>,
    {
        device
            .build_input_stream::<T, _, _>(
                config,
                move |data, _| {
                    if data.is_empty() {
                        return;
                    }
                    if data.len() > MAX_CALLBACK_SAMPLES {
                        overflowed.store(true, Ordering::Release);
                        return;
                    }
                    let converted = data
                        .iter()
                        .map(|sample| <T as cpal::Sample>::to_sample::<i16>(*sample))
                        .collect::<Vec<_>>();
                    if matches!(sender.try_send(converted), Err(TrySendError::Full(_))) {
                        overflowed.store(true, Ordering::Release);
                    }
                },
                move |_| stream_failed.store(true, Ordering::Release),
                None,
            )
            .map_err(|error| format!("Could not open audio input stream: {error}"))
    }

    impl CaptureSession {
        pub(crate) fn drain(&mut self) -> Result<(), String> {
            loop {
                match self.samples.try_recv() {
                    Ok(block) => self
                        .writer
                        .as_mut()
                        .ok_or_else(|| "Recording is already finalized".to_owned())?
                        .append(&block)?,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        return Err("Audio input stream disconnected".into())
                    }
                }
            }
            if self.overflowed.swap(false, Ordering::AcqRel) {
                return Err("Recording input buffer overflowed; a partial WAV was saved".into());
            }
            if self.stream_failed.swap(false, Ordering::AcqRel) {
                return Err("Audio input device failed; a partial WAV was saved".into());
            }
            Ok(())
        }

        pub(crate) fn duration_ms(&self) -> u64 {
            self.writer
                .as_ref()
                .map_or(0, Pcm16WavRecorder::duration_ms)
        }

        pub(crate) fn finish(mut self) -> Result<FinalizedRecording, String> {
            // Dropping the stream closes its callback sender before the final queue drain.
            drop(self.stream.take());
            let writer = self
                .writer
                .take()
                .ok_or_else(|| "Recording is already finalized".to_owned())?;
            let mut partial_reason = None;
            if self.overflowed.swap(false, Ordering::AcqRel) {
                add_partial_reason(
                    &mut partial_reason,
                    "input callback buffer overflowed; samples were dropped",
                );
            }
            if self.stream_failed.swap(false, Ordering::AcqRel) {
                add_partial_reason(&mut partial_reason, "input device reported a stream error");
            }
            // The stream is closed before draining. The helper preserves a valid written
            // prefix and records any queue write failure as an explicit partial result.
            finalize_pending_capture(writer, self.samples.try_iter(), partial_reason)
        }

        pub(crate) fn cancel(mut self) -> Result<String, String> {
            self.stream.take();
            let writer = self
                .writer
                .take()
                .ok_or_else(|| "Recording is already finalized".to_owned())?;
            writer.abort()?;
            Ok(self.path.clone())
        }
    }
}

#[cfg(feature = "audio-playback")]
pub(crate) use capture::{enumerate_inputs, start_capture, CaptureSession, InputDevice};

fn add_partial_reason(reason: &mut Option<String>, addition: &str) {
    match reason {
        Some(reason) => {
            reason.push_str("; ");
            reason.push_str(addition);
        }
        None => *reason = Some(addition.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_buffers_finalize_a_readable_pcm16_wav() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recorded.wav");
        let mut recorder = Pcm16WavRecorder::create(&path, 2, 48_000).unwrap();
        recorder.append(&[1_000, -1_000, 2_000, -2_000]).unwrap();
        let (finished_path, frames, sample_rate) = recorder.finish().unwrap();
        assert_eq!(finished_path, path);
        assert_eq!(frames, 2);
        assert_eq!(sample_rate, 48_000);
        let mut reader = hound::WavReader::open(path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(
            reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [1_000, -1_000, 2_000, -2_000]
        );
    }

    #[test]
    fn recording_refuses_overwrite_and_removes_aborted_new_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing.wav");
        fs::write(&path, b"preserve me").unwrap();
        assert!(Pcm16WavRecorder::create(&path, 1, 16_000).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"preserve me");

        let aborted = directory.path().join("aborted.wav");
        Pcm16WavRecorder::create(&aborted, 1, 16_000)
            .unwrap()
            .abort()
            .unwrap();
        assert!(!aborted.exists());
    }

    #[test]
    fn recording_rejects_partial_frames_and_invalid_formats() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mono.wav");
        let mut recorder = Pcm16WavRecorder::create(&path, 2, 44_100).unwrap();
        assert!(recorder.append(&[1]).is_err());
        assert!(Pcm16WavRecorder::create(&directory.path().join("wrong.mp3"), 1, 16_000).is_err());
        assert!(Pcm16WavRecorder::create(&directory.path().join("bad.wav"), 8, 192_000).is_err());
    }

    #[test]
    fn queued_write_failure_finalizes_valid_partial_wav_and_is_reported() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("partial.wav");
        let mut recorder = Pcm16WavRecorder::create_with_sample_limit(&path, 2, 48_000, 2).unwrap();
        recorder.append(&[100, -100]).unwrap();
        let finalized =
            finalize_pending_capture(recorder, [vec![200, -200], vec![300, -300]], None).unwrap();
        assert_eq!(finalized.path, path);
        assert_eq!(finalized.frames, 1);
        assert_eq!(finalized.sample_rate, 48_000);
        assert!(finalized
            .partial_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("safety limit")));

        let mut reader = hound::WavReader::open(finalized.path).unwrap();
        assert_eq!(reader.duration(), 1);
        assert_eq!(
            reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [100, -100]
        );
    }
}
