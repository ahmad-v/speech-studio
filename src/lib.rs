#![forbid(unsafe_code)]
use localwright_core::*;
#[cfg(feature = "asr")]
use sha2::{Digest, Sha256};
use std::{io::Cursor, path::Path};
pub const OPERATIONS:&[Operation]=&[
 Operation{id:"audio-info",label:"Audio inspection",hint:"Inspect local 16 kHz mono PCM16 WAV; maximum 10 minutes."},
 Operation{id:"prepare-wav",label:"Prepare WAV for speech recognition",hint:"Convert 16, 32 or 48 kHz mono/stereo PCM16 WAV to a new 16 kHz mono WAV."},
 Operation{id:"open-srt",label:"Open SRT",hint:"Load and validate UTF-8 SRT into editable draft."},
 Operation{id:"export-srt",label:"Save SRT",hint:"Validate editable draft, or load SRT from input if draft empty. New .srt output."},
 Operation{id:"export-txt",label:"Save text",hint:"Extract text from editable SRT draft or input SRT. New .txt output."},
 Operation{id:"export-vtt",label:"Save WebVTT",hint:"Export an SRT file or edited SRT draft as plain-text WebVTT cues. Choose a new .vtt output."},
 #[cfg(feature="asr")]
 Operation{id:"transcribe",label:"Transcribe (experimental)",hint:"16 kHz mono PCM16 WAV. Parameter: absolute local Whisper model path | expected SHA256. CPU only; no model included."}];
fn err(e: impl std::fmt::Display) -> Error {
    Error::input(e.to_string())
}
fn wav(p: &Path, j: &Job) -> Result<Vec<f32>> {
    let bytes = read_bounded(p, 20 * 1024 * 1024)?;
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(err)?;
    let s = reader.spec();
    if s.channels != 1
        || s.sample_rate != 16000
        || s.bits_per_sample != 16
        || s.sample_format != hound::SampleFormat::Int
    {
        return Err(Error::input("Expected mono 16000 Hz signed PCM16 WAV"));
    }
    if reader.duration() > 9_600_000 {
        return Err(Error::input("Audio duration exceeds 10 minutes"));
    }
    let mut samples = Vec::with_capacity(reader.duration() as usize);
    for (i, v) in reader.samples::<i16>().enumerate() {
        if i % 4096 == 0 {
            j.check()?;
        }
        samples.push(v.map_err(err)? as f32 / 32768.0);
    }
    if samples.is_empty() {
        return Err(Error::input("Empty audio"));
    }
    Ok(samples)
}
fn prepare_wav(input: &Path, output: &Path, job: &Job) -> Result<u32> {
    if output.extension().and_then(|ext| ext.to_str()) != Some("wav") {
        return Err(Error::input("Expected .wav output"));
    }
    let bytes = read_bounded(input, 128 * 1024 * 1024)?;
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(err)?;
    let spec = reader.spec();
    if !matches!(spec.channels, 1 | 2)
        || !matches!(spec.sample_rate, 16_000 | 32_000 | 48_000)
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(Error::input(
            "Expected mono or stereo PCM16 WAV at 16, 32 or 48 kHz",
        ));
    }
    let frames = reader.duration();
    if frames == 0 || frames > 600 * spec.sample_rate {
        return Err(Error::input(
            "Audio must be nonempty and no longer than 10 minutes",
        ));
    }
    let ratio = spec.sample_rate / 16_000;
    let output_frames = frames.div_ceil(ratio);
    write_new(output, |file| {
        let mut writer = hound::WavWriter::new(
            file,
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(err)?;
        let mut samples = reader.samples::<i16>();
        for frame in (0..frames).step_by(ratio as usize) {
            if frame % (ratio * 1024) == 0 {
                job.check()?;
            }
            let count = (frames - frame).min(ratio) * u32::from(spec.channels);
            let mut sum = 0i32;
            for _ in 0..count {
                let sample = samples
                    .next()
                    .ok_or_else(|| Error::input("Incomplete PCM frame"))?
                    .map_err(err)?;
                sum += i32::from(sample);
            }
            writer
                .write_sample((sum / count as i32) as i16)
                .map_err(err)?;
        }
        if samples.next().is_some() {
            return Err(Error::input("WAV contains an incomplete channel frame"));
        }
        job.check()?;
        writer.finalize().map_err(err)?;
        Ok(())
    })?;
    Ok(output_frames)
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Segment {
    start: u64,
    end: u64,
    text: String,
}
fn stamp(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3600000,
        (ms / 60000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}
fn parse_stamp(s: &str) -> Result<u64> {
    let v = s.split([':', ',']).collect::<Vec<_>>();
    if v.len() != 4
        || v.iter()
            .zip([2, 2, 2, 3])
            .any(|(a, n)| a.len() != n || !a.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(Error::input("Invalid SRT timestamp"));
    }
    let n = v
        .iter()
        .map(|x| x.parse::<u64>().map_err(err))
        .collect::<Result<Vec<_>>>()?;
    if n[1] >= 60 || n[2] >= 60 {
        return Err(Error::input("Invalid minute or second"));
    }
    Ok(n[0] * 3600000 + n[1] * 60000 + n[2] * 1000 + n[3])
}
fn parse(s: &str) -> Result<Vec<Segment>> {
    if s.len() > 2 * 1024 * 1024 {
        return Err(Error::input("Subtitle exceeds limit"));
    }
    let s = s.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut segments = vec![];
    for block in s.trim().split("\n\n") {
        if block.trim().is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let index = lines
            .next()
            .ok_or_else(|| Error::input("Missing index"))?
            .trim()
            .parse::<usize>()
            .map_err(err)?;
        if index != segments.len() + 1 {
            return Err(Error::input("Expected sequential SRT indices"));
        }
        let (start, end) = lines
            .next()
            .and_then(|x| x.split_once(" --> "))
            .ok_or_else(|| Error::input("Invalid SRT timing"))?;
        let (start, end) = (parse_stamp(start)?, parse_stamp(end)?);
        let text = lines.collect::<Vec<_>>().join("\n");
        if end <= start
            || text.trim().is_empty()
            || segments.last().is_some_and(|p: &Segment| p.start > start)
        {
            return Err(Error::input("Invalid segment duration, text or order"));
        }
        segments.push(Segment { start, end, text });
    }
    if segments.is_empty() {
        return Err(Error::input("No subtitle segments"));
    }
    Ok(segments)
}
fn render(segments: &[Segment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                stamp(s.start),
                stamp(s.end),
                s.text
            )
        })
        .collect()
}
fn render_vtt(segments: &[Segment], job: &Job) -> Result<String> {
    use std::fmt::Write as _;
    let mut content = String::from("WEBVTT\n\n");
    for segment in segments {
        job.check()?;
        let start = stamp(segment.start).replace(',', ".");
        let end = stamp(segment.end).replace(',', ".");
        let text = segment
            .text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        writeln!(content, "{start} --> {end}\n{text}\n").expect("writing to a String cannot fail");
    }
    Ok(content)
}

#[cfg(feature = "asr")]
fn transcribe(samples: &[f32], parameter: &str, j: &Job) -> Result<Vec<Segment>> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
    let (path, expected) = parameter
        .rsplit_once('|')
        .ok_or_else(|| Error::input("Expected model path | SHA256"))?;
    let p = Path::new(path.trim());
    if !p.is_absolute() {
        return Err(Error::input("Model path must be absolute"));
    }
    let bytes = read_bounded(p, 512 * 1024 * 1024)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        return Err(Error::input("Model SHA256 mismatch"));
    }
    j.check()?;
    if samples.iter().all(|s| s.abs() < 0.0001) {
        return Err(Error::input("Audio is silent; no transcription attempted"));
    }
    // Keep native decoder output out of the console and application logs.
    whisper_rs::install_logging_hooks();
    let mut cp = WhisperContextParameters::default();
    cp.use_gpu(false);
    let ctx = WhisperContext::new_from_buffer_with_params(&bytes, cp)
        .map_err(|e| Error::engine(e.to_string()))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| Error::engine(e.to_string()))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(2);
    params.set_language(Some("auto"));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    params.set_translate(false);
    state
        .full(params, samples)
        .map_err(|e| Error::engine(e.to_string()))?;
    j.check()?;
    let mut out = vec![];
    let count = state
        .full_n_segments()
        .map_err(|e| Error::engine(e.to_string()))?;
    for i in 0..count {
        j.check()?;
        let start = state
            .full_get_segment_t0(i)
            .map_err(|e| Error::engine(e.to_string()))?;
        let end = state
            .full_get_segment_t1(i)
            .map_err(|e| Error::engine(e.to_string()))?;
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| Error::engine(e.to_string()))?
            .trim()
            .to_owned();
        if start >= 0 && end > start && !text.is_empty() {
            out.push(Segment {
                start: start as u64 * 10,
                end: end as u64 * 10,
                text,
            });
        }
    }
    if out.is_empty() {
        return Err(Error::engine("No speech segments returned"));
    }
    Ok(out)
}
pub fn handle(r: Request, j: Job) -> Result<Outcome> {
    j.check()?;
    let mut out = Outcome::default();
    match r.operation.as_str() {
        "prepare-wav" => {
            let frames = prepare_wav(&r.input, &r.output, &j)?;
            out.preview = format!("Prepared {frames} mono samples at 16,000 Hz");
            out.written = Some(r.output);
        }
        "audio-info" => {
            let s = wav(&r.input, &j)?;
            out.preview = format!(
                "PCM16 mono, 16000 Hz, {} samples, {:.3} seconds",
                s.len(),
                s.len() as f64 / 16000.0
            );
        }
        "open-srt" | "export-srt" | "export-txt" | "export-vtt" => {
            let source = if r.operation == "open-srt" || r.draft.trim().is_empty() {
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?
            } else {
                r.draft
            };
            let segments = parse(&source)?;
            let normalized = render(&segments);
            if r.operation == "open-srt" {
                out.draft = Some(normalized);
            } else {
                let (ext, content) = match r.operation.as_str() {
                    "export-srt" => ("srt", normalized),
                    "export-vtt" => ("vtt", render_vtt(&segments, &j)?),
                    _ => (
                        "txt",
                        segments
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                };
                if r.output.extension().and_then(|x| x.to_str()) != Some(ext) {
                    return Err(Error::input(format!("Expected .{ext} output")));
                }
                j.check()?;
                write_bytes(&r.output, content.as_bytes())?;
                out.written = Some(r.output);
            }
            out.preview = format!("{} subtitle segments", segments.len());
        }
        #[cfg(feature = "asr")]
        "transcribe" => {
            let samples = wav(&r.input, &j)?;
            let segments = transcribe(&samples, &r.parameter, &j)?;
            let s = render(&segments);
            parse(&s)?;
            out.preview = format!(
                "{} experimental segments; review before export",
                segments.len()
            );
            out.draft = Some(s);
            out.warnings.push("No accuracy, silence robustness or language support verified. Cancellation is cooperative before/after native inference, not during it.".into());
        }
        _ => return Err(Error::input("Unknown or unavailable operation")),
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let s =
            "1\n00:00:00,000 --> 00:00:01,500\nCafé\n\n2\n00:00:01,500 --> 00:00:02,000\nWorld\n";
        let a = parse(s).unwrap();
        assert_eq!(parse(&render(&a)).unwrap(), a);
        assert!(parse("1\n00:00:02,000 --> 00:00:01,000\nx").is_err());
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("out.txt");
        handle(
            Request {
                operation: "export-txt".into(),
                draft: s.into(),
                output: p.clone(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(p).unwrap(), "Café\nWorld");
    }
    #[test]
    fn webvtt_export_preserves_plain_text_and_timing() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("out.vtt");
        handle(
            Request {
                operation: "export-vtt".into(),
                output: output.clone(),
                draft: "1\n00:00:00,500 --> 00:00:02,100\nCafé & <text> --> here".into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            "WEBVTT\n\n00:00:00.500 --> 00:00:02.100\nCafé &amp; &lt;text&gt; --&gt; here\n\n"
        );
    }
    #[test]
    fn prepares_stereo_wav_with_a_partial_final_group() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.wav");
        let output = dir.path().join("output.wav");
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 2,
                sample_rate: 32_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in [1000i16, 3000, -1000, 1000, 2000, 4000] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        assert_eq!(prepare_wav(&input, &output, &Job::new()).unwrap(), 2);
        let mut result = hound::WavReader::open(output).unwrap();
        assert_eq!(result.spec().channels, 1);
        assert_eq!(result.spec().sample_rate, 16_000);
        assert_eq!(
            result
                .samples::<i16>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap(),
            vec![1000, 3000]
        );
    }

    #[test]
    fn audio() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        w.write_sample(16384i16).unwrap();
        w.finalize().unwrap();
        assert_eq!(wav(&p, &Job::new()).unwrap(), vec![0.5]);
    }
}
