#![forbid(unsafe_code)]

use crate::recording::{enumerate_inputs, start_capture, CaptureSession, InputDevice};
use crate::{
    audio_model::{
        active_cue_at_audio_position, audio_position_for_cue, clamp_seek, decode_workspace_project,
        encode_workspace_project, format_time, matching_cue_indices, parse_transcript_offset,
        transport_action, waveform_seek, CueHistory, DecodedWorkspaceProject, TransportAction,
    },
    load_playback_clip, open_audio_decoder, parse_stamp, stamp, PlaybackClip, Segment,
};
use iced::{
    widget::{
        button, canvas, column, container, pick_list, row, scrollable, slider, text, text_editor,
        text_input,
    },
    Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme,
};
use localwright_core::Job;
use localwright_core::{read_bounded, write_bytes};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::{
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

const CUES_PER_PAGE: usize = 25;

#[cfg(feature = "asr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsrModel {
    EnglishTiny,
    MultilingualTiny,
}

#[cfg(feature = "asr")]
impl std::fmt::Display for AsrModel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EnglishTiny => "English Tiny",
            Self::MultilingualTiny => "Multilingual Tiny",
        })
    }
}

#[cfg(feature = "asr")]
impl AsrModel {
    fn parameter(self) -> &'static str {
        match self {
            Self::EnglishTiny => "",
            Self::MultilingualTiny => "multilingual",
        }
    }
}

struct Waveform<'a> {
    peaks: &'a [f32],
    position_ms: u64,
    duration_ms: u64,
}

impl canvas::Program<Message> for Waveform<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: canvas::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> (iced::event::Status, Option<Message>) {
        if let canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) =
            event
        {
            if let Some(position) = cursor.position_in(bounds) {
                if let Some(milliseconds) =
                    waveform_seek(position.x, bounds.width, self.duration_ms)
                {
                    return (
                        iced::event::Status::Captured,
                        Some(Message::WaveformSeek(milliseconds)),
                    );
                }
            }
        }
        (iced::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = frame.width();
        let height = frame.height();
        let middle = height / 2.0;
        let baseline = canvas::Path::line(Point::new(0.0, middle), Point::new(width, middle));
        frame.stroke(
            &baseline,
            canvas::Stroke::default()
                .with_color(Color::from_rgb8(125, 135, 145))
                .with_width(1.0),
        );
        if !self.peaks.is_empty() {
            let bars = canvas::Path::new(|builder| {
                for (index, peak) in self.peaks.iter().copied().enumerate() {
                    let x = (index as f32 + 0.5) * width / self.peaks.len() as f32;
                    let amplitude = if peak.is_finite() {
                        peak.abs().clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let extent = amplitude * (height * 0.46);
                    builder.move_to(Point::new(x, middle - extent));
                    builder.line_to(Point::new(x, middle + extent));
                }
            });
            frame.stroke(
                &bars,
                canvas::Stroke::default()
                    .with_color(Color::from_rgb8(55, 125, 185))
                    .with_width((width / self.peaks.len() as f32).clamp(1.0, 3.0)),
            );
        }
        if self.duration_ms > 0 {
            let ratio = (self.position_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0);
            let x = ratio * width;
            let playhead = canvas::Path::line(Point::new(x, 0.0), Point::new(x, height));
            frame.stroke(
                &playhead,
                canvas::Stroke::default()
                    .with_color(Color::from_rgb8(220, 65, 55))
                    .with_width(2.0),
            );
        }
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Default)]
struct Snapshot {
    file_name: String,
    audio_path: Option<PathBuf>,
    waveform: Vec<f32>,
    duration_ms: u64,
    position_ms: u64,
    loaded: bool,
    loading: bool,
    playing: bool,
    paused: bool,
    input_devices: Vec<String>,
    selected_input: Option<String>,
    recording: bool,
    recording_ms: u64,
    message: String,
}

enum Command {
    Open(PathBuf),
    Play,
    Pause,
    Resume,
    Stop,
    RefreshInputs,
    SelectInput(usize),
    StartRecording(PathBuf),
    StopRecording,
    CancelRecording,
    Seek(u64),
    Shutdown,
}

struct Session {
    sink: Sink,
    _stream: OutputStream,
}

pub fn run() -> iced::Result {
    iced::application(
        "Speech Studio — Audio workspace",
        State::update,
        State::view,
    )
    .subscription(State::subscription)
    .run_with(State::new)
}

struct State {
    commands: mpsc::Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    view: Snapshot,
    seek_value_ms: Option<f32>,
    cues: CueHistory,
    selected_cue: Option<usize>,
    page_start: usize,
    cue_start_text: String,
    cue_end_text: String,
    transcript_offset_text: String,
    cue_text: text_editor::Content,
    find_text: String,
    replacement_text: String,
    linked_audio: Option<PathBuf>,
    notice: String,
    #[cfg(feature = "asr")]
    asr_model: AsrModel,
    #[cfg(feature = "asr")]
    transcription_job: Job,
    #[cfg(feature = "asr")]
    transcribing: bool,
    #[cfg(feature = "asr")]
    model_check_job: Job,
    #[cfg(feature = "asr")]
    model_checking: bool,
    #[cfg(feature = "asr")]
    model_status: Option<std::result::Result<String, String>>,
}

impl State {
    fn new() -> (Self, Task<Message>) {
        let snapshot = Arc::new(Mutex::new(Snapshot {
            message: "Open a supported local audio file to begin.".into(),
            ..Snapshot::default()
        }));
        let (commands, receiver) = mpsc::channel();
        let worker_snapshot = Arc::clone(&snapshot);
        if let Err(error) = thread::Builder::new()
            .name("speech-audio-player".into())
            .spawn(move || audio_worker(receiver, worker_snapshot))
        {
            lock_snapshot(&snapshot).message = format!("Could not start audio worker: {error}");
        } else {
            let _ = commands.send(Command::RefreshInputs);
        }
        let view = lock_snapshot(&snapshot).clone();
        #[cfg(feature = "asr")]
        let model_check_job = Job::new();
        #[cfg(feature = "asr")]
        let initial_task = model_status_task(AsrModel::EnglishTiny, model_check_job.clone());
        #[cfg(not(feature = "asr"))]
        let initial_task = Task::none();
        (
            Self {
                commands,
                snapshot,
                view,
                seek_value_ms: None,
                cues: CueHistory::new(Vec::new()),
                selected_cue: None,
                page_start: 0,
                cue_start_text: String::new(),
                cue_end_text: String::new(),
                transcript_offset_text: "0".into(),
                cue_text: text_editor::Content::new(),
                find_text: String::new(),
                replacement_text: String::new(),
                linked_audio: None,
                notice: String::new(),
                #[cfg(feature = "asr")]
                asr_model: AsrModel::EnglishTiny,
                #[cfg(feature = "asr")]
                transcription_job: Job::new(),
                #[cfg(feature = "asr")]
                transcribing: false,
                #[cfg(feature = "asr")]
                model_check_job,
                #[cfg(feature = "asr")]
                model_checking: true,
                #[cfg(feature = "asr")]
                model_status: None,
            },
            initial_task,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenDialog => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open local audio")
                            .add_filter(
                                "Audio (WAV, FLAC, MP3, OGG, AAC, AIFF)",
                                &["wav", "flac", "mp3", "ogg", "aac", "m4a", "aiff"],
                            )
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::FilePicked,
                );
            }
            Message::FilePicked(Some(path)) => {
                #[cfg(feature = "asr")]
                {
                    self.transcription_job.cancel();
                    self.transcribing = false;
                }
                self.seek_value_ms = None;
                self.linked_audio = None;
                self.notice.clear();
                if self.commands.send(Command::Open(path)).is_ok() {
                    self.view.loading = true;
                    self.view.message = "Reading and validating audio…".into();
                } else {
                    self.view.message = "The audio worker has stopped.".into();
                }
            }
            Message::FilePicked(None) => {}
            #[cfg(feature = "asr")]
            Message::AsrModelChanged(model) => {
                self.model_check_job.cancel();
                self.model_check_job = Job::new();
                self.asr_model = model;
                self.model_status = None;
                self.model_checking = true;
                return model_status_task(model, self.model_check_job.clone());
            }
            #[cfg(feature = "asr")]
            Message::ModelStatusChecked(model, result) => {
                if self.asr_model == model {
                    self.model_checking = false;
                    self.model_status = Some(result.clone());
                    self.notice = match result {
                        Ok(status) => format!("{}: {status}.", model),
                        Err(error) => format!("{} is unavailable: {error}.", model),
                    };
                }
            }
            #[cfg(feature = "asr")]
            Message::Transcribe => {
                if self.transcribing {
                    self.notice = "A transcription job is already active.".into();
                } else if self.model_checking
                    || !self
                        .model_status
                        .as_ref()
                        .is_some_and(|status| status.is_ok())
                {
                    self.notice = "Wait for the selected model pack to finish verification.".into();
                } else if let Some(path) = self.view.audio_path.clone() {
                    let job = Job::new();
                    self.transcription_job = job.clone();
                    self.transcribing = true;
                    self.notice = format!(
                        "Transcribing with {}. Existing cues are replaced when the result is ready; Undo restores them.",
                        self.asr_model
                    );
                    return transcribe_task(path, self.asr_model, false, job);
                } else {
                    self.notice = "Open audio before starting transcription.".into();
                }
            }
            #[cfg(feature = "asr")]
            Message::TranslateToEnglish => {
                if self.transcribing {
                    self.notice = "A speech job is already active.".into();
                } else if self.asr_model != AsrModel::MultilingualTiny {
                    self.notice =
                        "Speech-to-English translation requires the Multilingual Tiny model."
                            .into();
                } else if self.model_checking
                    || !self
                        .model_status
                        .as_ref()
                        .is_some_and(|status| status.is_ok())
                {
                    self.notice =
                        "Wait for the multilingual model pack to finish verification.".into();
                } else if let Some(path) = self.view.audio_path.clone() {
                    let job = Job::new();
                    self.transcription_job = job.clone();
                    self.transcribing = true;
                    self.notice = "Translating speech to English with the offline multilingual model. Existing cues are replaced when the result is ready; Undo restores them.".into();
                    return transcribe_task(path, self.asr_model, true, job);
                } else {
                    self.notice = "Open audio before translating speech.".into();
                }
            }
            #[cfg(feature = "asr")]
            Message::CancelTranscription => {
                self.transcription_job.cancel();
                self.notice = "Cancellation requested. Whisper checks it after native inference, so the current inference call may finish first.".into();
            }
            #[cfg(feature = "asr")]
            Message::TranscriptionFinished(source, translated, result) => {
                self.transcribing = false;
                if self.view.audio_path.as_deref() != Some(source.as_path()) {
                    self.notice =
                        "Discarded transcription because the loaded audio changed.".into();
                } else {
                    match result {
                        Ok(segments) => {
                            let count = segments.len();
                            match self.cues.replace_all(segments) {
                                Ok(()) => {
                                    self.selected_cue = None;
                                    self.page_start = 0;
                                    self.cue_start_text.clear();
                                    self.cue_end_text.clear();
                                    self.cue_text = text_editor::Content::new();
                                    self.transcript_offset_text = "0".into();
                                    self.find_text.clear();
                                    self.notice = if translated {
                                        format!("Created {count} experimental English translation cues. Review the translation and timestamps; language quality has not been established.")
                                    } else {
                                        format!("Created {count} experimental transcript cues. Review the transcript and timestamps; WER/CER and language accuracy are not established.")
                                    };
                                }
                                Err(error) => {
                                    self.notice = format!("Transcription was not applied: {error}");
                                }
                            }
                        }
                        Err(error) => {
                            self.notice = if translated {
                                format!("Speech translation failed: {error}")
                            } else {
                                format!("Transcription failed: {error}")
                            };
                        }
                    }
                }
            }
            Message::RecordDialog => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Save a new WAV recording")
                            .add_filter("WAV audio", &["wav"])
                            .save_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::RecordPath,
                );
            }
            Message::RecordPath(Some(path)) => {
                self.notice.clear();
                self.send(Command::StartRecording(path));
            }
            Message::RecordPath(None) => {}
            Message::RefreshInputs => self.send(Command::RefreshInputs),
            Message::InputSelected(label) => {
                if let Some((index, _)) = label.split_once(": ") {
                    if let Ok(index) = index.parse::<usize>() {
                        self.send(Command::SelectInput(index));
                    }
                }
            }
            Message::StopRecording => self.send(Command::StopRecording),
            Message::CancelRecording => self.send(Command::CancelRecording),
            Message::OpenTranscript => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open SRT transcript or transcript project")
                            .add_filter("SRT or JSON transcript", &["srt", "json"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::TranscriptPicked,
                );
            }
            Message::TranscriptPicked(Some(path)) => match load_transcript(&path) {
                Ok(project) => {
                    let count = project.segments.len();
                    let crate::audio_model::DecodedWorkspaceProject {
                        segments,
                        audio_path,
                        transcript_offset_ms,
                    } = project;
                    self.cues = CueHistory::with_transcript_offset(segments, transcript_offset_ms);
                    self.linked_audio = audio_path.map(|media| {
                        if media.is_absolute() {
                            media
                        } else {
                            path.parent().unwrap_or_else(|| Path::new(".")).join(media)
                        }
                    });
                    self.selected_cue = None;
                    self.page_start = 0;
                    self.cue_start_text.clear();
                    self.cue_end_text.clear();
                    self.transcript_offset_text = transcript_offset_ms.to_string();
                    self.cue_text = text_editor::Content::new();
                    self.find_text.clear();
                    self.replacement_text.clear();
                    self.notice = if let Some(audio) = &self.linked_audio {
                        format!(
                            "Loaded {count} cues. Linked audio is available at {}",
                            audio.display()
                        )
                    } else {
                        format!("Loaded {count} transcript cues.")
                    };
                }
                Err(error) => self.notice = format!("Could not open transcript: {error}"),
            },
            Message::TranscriptPicked(None) => {}
            Message::SaveProject => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Save transcript project as a new file")
                            .add_filter("JSON transcript project", &["json"])
                            .save_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::ProjectSavePath,
                );
            }
            Message::ProjectSavePath(Some(path)) => {
                match save_project(
                    &path,
                    self.cues.cues(),
                    self.view.audio_path.as_deref(),
                    self.cues.transcript_offset_ms(),
                ) {
                    Ok(()) => {
                        self.linked_audio = self.view.audio_path.as_ref().and_then(|audio| {
                            relative_media_path(&path, audio).ok().map(PathBuf::from)
                        });
                        self.notice = format!("Saved audio workspace project to {}", path.display())
                    }
                    Err(error) => self.notice = format!("Could not save project: {error}"),
                }
            }
            Message::ProjectSavePath(None) => {}
            Message::LoadLinkedAudio => {
                if let Some(path) = self.linked_audio.clone() {
                    self.linked_audio = None;
                    if self.commands.send(Command::Open(path)).is_ok() {
                        self.view.loading = true;
                        self.view.message = "Reading linked audio…".into();
                    } else {
                        self.view.message = "The audio worker has stopped.".into();
                    }
                }
            }
            Message::SelectCue(index) => {
                if let Some(cue) = self.cues.cues().get(index) {
                    let (start, end, text) = (cue.start, cue.end, cue.text.clone());
                    self.notice.clear();
                    self.selected_cue = Some(index);
                    if self.find_text.is_empty() {
                        self.page_start = index / CUES_PER_PAGE * CUES_PER_PAGE;
                    }
                    self.cue_start_text = stamp(start);
                    self.cue_end_text = stamp(end);
                    self.cue_text = text_editor::Content::with_text(&text);
                    let audio_position =
                        audio_position_for_cue(start, self.cues.transcript_offset_ms())
                            .min(self.view.duration_ms);
                    self.send(Command::Seek(audio_position));
                }
            }
            Message::CuePagePrevious => {
                self.page_start = self.page_start.saturating_sub(CUES_PER_PAGE);
            }
            Message::CuePageNext => {
                let matching_count = matching_cue_indices(self.cues.cues(), &self.find_text).len();
                if self.page_start + CUES_PER_PAGE < matching_count {
                    self.page_start += CUES_PER_PAGE;
                }
            }
            Message::CueStartChanged(value) => self.cue_start_text = value,
            Message::CueEndChanged(value) => self.cue_end_text = value,
            Message::TranscriptOffsetChanged(value) => self.transcript_offset_text = value,
            Message::ApplyTranscriptOffset => {
                match parse_transcript_offset(&self.transcript_offset_text) {
                    Ok(offset) => match self.cues.set_transcript_offset(offset) {
                        Ok(()) => {
                            self.transcript_offset_text = offset.to_string();
                            self.notice = format!("Transcript offset applied: {offset} ms.");
                        }
                        Err(error) => {
                            self.notice = format!("Transcript offset was not changed: {error}");
                        }
                    },
                    Err(error) => {
                        self.notice = format!("Transcript offset was not changed: {error}");
                    }
                }
            }
            Message::CueTextAction(action) => self.cue_text.perform(action),
            Message::ApplyCueText => {
                if let Some(index) = self.selected_cue {
                    let text = self.cue_text.text();
                    match self.cues.set_text(index, &text) {
                        Ok(()) => self.notice = "Cue text updated.".into(),
                        Err(error) => {
                            self.cue_text =
                                text_editor::Content::with_text(&self.cues.cues()[index].text);
                            self.notice = format!("Cue text was not changed: {error}");
                        }
                    }
                } else {
                    self.notice = "Select a cue before editing its text.".into();
                }
            }
            Message::FindTextChanged(value) => {
                self.find_text = value;
                self.page_start = 0;
            }
            Message::ReplacementTextChanged(value) => self.replacement_text = value,
            Message::ReplaceSelected => {
                let result = self.cues.replace_text(
                    self.selected_cue,
                    &self.find_text,
                    &self.replacement_text,
                );
                match result {
                    Ok(count) => {
                        self.refresh_selected_cue();
                        self.notice =
                            format!("Replaced {count} occurrence(s) in the selected cue.");
                    }
                    Err(error) => self.notice = format!("Text was not replaced: {error}"),
                }
            }
            Message::ReplaceAll => {
                match self
                    .cues
                    .replace_text(None, &self.find_text, &self.replacement_text)
                {
                    Ok(count) => {
                        self.refresh_selected_cue();
                        self.notice = format!("Replaced {count} occurrence(s) in the transcript.");
                    }
                    Err(error) => self.notice = format!("Text was not replaced: {error}"),
                }
            }
            Message::ApplyCueTiming => {
                if let Some(index) = self.selected_cue {
                    let timing = parse_stamp(&self.cue_start_text)
                        .and_then(|start| parse_stamp(&self.cue_end_text).map(|end| (start, end)));
                    match timing {
                        Ok((start, end)) => match self.cues.set_timing(index, start, end) {
                            Ok(()) => {
                                self.cue_start_text = stamp(start);
                                self.cue_end_text = stamp(end);
                                self.notice = "Cue timing updated.".into();
                            }
                            Err(error) => self.notice = format!("Invalid cue timing: {error}"),
                        },
                        Err(error) => {
                            self.notice = format!("Invalid SRT timestamp: {}", error.message)
                        }
                    }
                } else {
                    self.notice = "Select a cue before editing its timing.".into();
                }
            }
            Message::UndoCueTiming => {
                self.notice = if self.cues.undo() {
                    self.refresh_selected_timing();
                    self.transcript_offset_text = self.cues.transcript_offset_ms().to_string();
                    "Transcript edit undone.".into()
                } else {
                    "No transcript edit to undo.".into()
                };
            }
            Message::RedoCueTiming => {
                self.notice = if self.cues.redo() {
                    self.refresh_selected_timing();
                    self.transcript_offset_text = self.cues.transcript_offset_ms().to_string();
                    "Transcript edit redone.".into()
                } else {
                    "No transcript edit to redo.".into()
                };
            }
            Message::Transport => {
                self.notice.clear();
                match transport_action(self.view.loaded, self.view.playing, self.view.paused) {
                    Some(TransportAction::Play) => self.send(Command::Play),
                    Some(TransportAction::Pause) => self.send(Command::Pause),
                    Some(TransportAction::Resume) => self.send(Command::Resume),
                    None => {}
                }
            }
            Message::Stop => {
                self.notice.clear();
                self.send(Command::Stop);
            }
            Message::Scrub(value) => self.seek_value_ms = Some(value),
            Message::SeekReleased => {
                if let Some(value) = self.seek_value_ms.take() {
                    self.notice.clear();
                    self.send(Command::Seek(clamp_seek(value, self.view.duration_ms)));
                }
            }
            Message::WaveformSeek(position) => {
                self.seek_value_ms = None;
                self.notice.clear();
                if self.view.loaded {
                    self.send(Command::Seek(position.min(self.view.duration_ms)));
                }
            }
            Message::Tick => {
                self.view = lock_snapshot(&self.snapshot).clone();
                if self.find_text.is_empty() {
                    if let Some(active) = active_cue_at_audio_position(
                        self.cues.cues(),
                        self.view.position_ms,
                        self.cues.transcript_offset_ms(),
                    ) {
                        let page_start = active / CUES_PER_PAGE * CUES_PER_PAGE;
                        if page_start != self.page_start {
                            self.page_start = page_start;
                        }
                    }
                }
            }
        }
        Task::none()
    }

    fn send(&mut self, command: Command) {
        if self.commands.send(command).is_err() {
            self.view.message = "The audio worker has stopped.".into();
        }
    }

    fn refresh_selected_timing(&mut self) {
        if let Some(cue) = self
            .selected_cue
            .and_then(|index| self.cues.cues().get(index))
        {
            self.cue_start_text = stamp(cue.start);
            self.cue_end_text = stamp(cue.end);
            self.cue_text = text_editor::Content::with_text(&cue.text);
        }
    }

    fn refresh_selected_cue(&mut self) {
        self.refresh_selected_timing();
    }

    fn view(&self) -> Element<'_, Message> {
        let action = transport_action(self.view.loaded, self.view.playing, self.view.paused);
        let action_label = match action {
            Some(TransportAction::Pause) => "Pause",
            Some(TransportAction::Resume) => "Resume",
            Some(TransportAction::Play) => "Play",
            None => "Play",
        };
        let position = self
            .seek_value_ms
            .map(|value| clamp_seek(value, self.view.duration_ms))
            .unwrap_or(self.view.position_ms);
        let duration = self.view.duration_ms.max(1);
        let waveform = canvas::Canvas::new(Waveform {
            peaks: &self.view.waveform,
            position_ms: position,
            duration_ms: self.view.duration_ms,
        })
        .width(Length::Fill)
        .height(Length::Fixed(160.0));
        let cues = self.cues.cues();
        let active = active_cue_at_audio_position(
            cues,
            self.view.position_ms,
            self.cues.transcript_offset_ms(),
        );
        let matching_indices = matching_cue_indices(cues, &self.find_text);
        let visible_cues = matching_indices
            .iter()
            .skip(self.page_start)
            .take(CUES_PER_PAGE)
            .copied()
            .collect::<Vec<_>>();
        let cue_rows = visible_cues
            .iter()
            .map(|index| {
                let cue = &cues[*index];
                let marker = if active == Some(*index) {
                    "▶"
                } else if self.selected_cue == Some(*index) {
                    "●"
                } else {
                    " "
                };
                let cue_text = cue.text.chars().take(120).collect::<String>();
                button(text(format!(
                    "{marker} {:>4}  {}–{}  {cue_text}",
                    *index + 1,
                    format_time(cue.start),
                    format_time(cue.end),
                )))
                .width(Length::Fill)
                .on_press(Message::SelectCue(*index))
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();
        let cue_list = scrollable(column(cue_rows).spacing(4)).height(Length::Fixed(240.0));
        let visible_end = (self.page_start + visible_cues.len()).min(matching_indices.len());
        let visible_range = if matching_indices.is_empty() {
            if cues.is_empty() {
                "No transcript loaded".into()
            } else {
                "No cues match this search".into()
            }
        } else if !self.find_text.is_empty() {
            format!(
                "Matches {}–{visible_end} of {} filtered cues ({} total)",
                self.page_start + 1,
                matching_indices.len(),
                cues.len(),
            )
        } else {
            format!(
                "Cues {}–{visible_end} of {}",
                self.page_start + 1,
                matching_indices.len()
            )
        };
        #[cfg(feature = "asr")]
        let model_status = if self.model_checking {
            format!("Verifying {} beside the application…", self.asr_model)
        } else {
            match self.model_status.as_ref() {
                Some(Ok(status)) => status.clone(),
                Some(Err(error)) => format!("Pack unavailable: {error}"),
                None => "Model pack status has not been checked.".into(),
            }
        };
        #[cfg(feature = "asr")]
        let model_ready = !self.model_checking
            && self
                .model_status
                .as_ref()
                .is_some_and(|status| status.is_ok());
        #[cfg(feature = "asr")]
        let model_message = if self.transcribing {
            "Speech processing runs in the background; Whisper cancellation is checked after native inference."
                .to_owned()
        } else {
            model_status
        };
        #[cfg(feature = "asr")]
        let asr_controls = row![
            text("Offline ASR model"),
            pick_list(
                [AsrModel::EnglishTiny, AsrModel::MultilingualTiny],
                Some(self.asr_model),
                Message::AsrModelChanged,
            ),
            button(if self.transcribing {
                "Cancel speech job"
            } else {
                "Transcribe loaded audio"
            })
            .on_press_maybe(if self.transcribing {
                Some(Message::CancelTranscription)
            } else if self.view.loaded && model_ready {
                Some(Message::Transcribe)
            } else {
                None
            }),
            button("Translate speech to English").on_press_maybe(if self.transcribing {
                None
            } else if self.view.loaded
                && self.asr_model == AsrModel::MultilingualTiny
                && model_ready
            {
                Some(Message::TranslateToEnglish)
            } else {
                None
            }),
            text(model_message),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .wrap();

        scrollable(
            container({
                let controls = column![
                    row![
                        text("Speech Studio audio workspace").size(24),
                        button("Open audio…").on_press(Message::OpenDialog),
                    ]
                    .spacing(16)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    text(if self.view.file_name.is_empty() {
                        "No audio file open".to_owned()
                    } else {
                        self.view.file_name.clone()
                    }),
                    waveform,
                    slider(0.0..=duration as f32, position as f32, Message::Scrub,)
                        .step(10.0)
                        .on_release(Message::SeekReleased)
                        .width(Length::Fill),
                    row![
                        text(format_time(position)),
                        text(" / "),
                        text(format_time(self.view.duration_ms)),
                    ]
                    .spacing(4),
                    row![
                        text_input("Filter transcript cues / find text", &self.find_text)
                            .on_input(Message::FindTextChanged),
                        text(visible_range.clone()),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    row![
                        text(format!(
                            "Transcript: {} cues · active cue {}",
                            cues.len(),
                            active.map_or_else(|| "—".into(), |index| (index + 1).to_string())
                        )),
                        button("Open SRT / project…").on_press(Message::OpenTranscript),
                        button("Save project…")
                            .on_press_maybe((!cues.is_empty()).then_some(Message::SaveProject)),
                        button("Load linked audio").on_press_maybe(
                            self.linked_audio.as_ref().map(|_| Message::LoadLinkedAudio)
                        ),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    row![
                        text("Transcript offset (ms; positive shifts cues later)"),
                        text_input("0", &self.transcript_offset_text)
                            .on_input(Message::TranscriptOffsetChanged),
                        button("Apply offset").on_press(Message::ApplyTranscriptOffset),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    row![
                        button("Undo")
                            .on_press_maybe(self.cues.can_undo().then_some(Message::UndoCueTiming)),
                        button("Redo")
                            .on_press_maybe(self.cues.can_redo().then_some(Message::RedoCueTiming)),
                        button("‹ Cues").on_press_maybe(
                            (self.page_start > 0).then_some(Message::CuePagePrevious)
                        ),
                        button("Cues ›").on_press_maybe(
                            (self.page_start + CUES_PER_PAGE < matching_indices.len())
                                .then_some(Message::CuePageNext)
                        ),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    cue_list,
                    text("Selected cue text (multi-line)").size(16),
                    text_editor(&self.cue_text)
                        .placeholder("Select a cue to edit its text")
                        .height(Length::Fixed(120.0))
                        .on_action(Message::CueTextAction),
                    button("Apply cue text")
                        .on_press_maybe(self.selected_cue.map(|_| Message::ApplyCueText)),
                    row![
                        text_input("Replace with", &self.replacement_text)
                            .on_input(Message::ReplacementTextChanged),
                        button("Replace selected")
                            .on_press_maybe(self.selected_cue.map(|_| Message::ReplaceSelected)),
                        button("Replace all")
                            .on_press_maybe((!self.find_text.is_empty()).then_some(Message::ReplaceAll)),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    text("Search and replace are case-sensitive literal text operations. Find filters the cue list; all edits preserve cue times and support Undo/Redo."),
                    row![
                        text("Selected cue start"),
                        text_input("HH:MM:SS,mmm", &self.cue_start_text)
                            .on_input(Message::CueStartChanged),
                        text("end"),
                        text_input("HH:MM:SS,mmm", &self.cue_end_text)
                            .on_input(Message::CueEndChanged),
                        button("Apply timing")
                            .on_press_maybe(self.selected_cue.map(|_| Message::ApplyCueTiming)),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    row![
                        pick_list(
                            self.view.input_devices.clone(),
                            self.view.selected_input.clone(),
                            Message::InputSelected,
                        )
                        .placeholder("No input device"),
                        button("Refresh inputs").on_press(Message::RefreshInputs),
                        button("Record to WAV…").on_press_maybe(
                            (!self.view.recording).then_some(Message::RecordDialog)
                        ),
                        button("Stop recording")
                            .on_press_maybe(self.view.recording.then_some(Message::StopRecording)),
                        button("Discard recording").on_press_maybe(
                            self.view.recording.then_some(Message::CancelRecording),
                        ),
                        text(if self.view.recording {
                            format!("Recording · {}", format_time(self.view.recording_ms))
                        } else {
                            "Ready to record".into()
                        }),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    row![
                        button(action_label).on_press_maybe(action.map(|_| Message::Transport)),
                        button("Stop").on_press_maybe(self.view.loaded.then_some(Message::Stop)),
                        text(if !self.notice.is_empty() {
                            self.notice.clone()
                        } else if self.view.loading {
                            "Loading audio…".to_owned()
                        } else {
                            self.view.message.clone()
                        })
                    ]
                    .spacing(12)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    text("Local processing · playback and recording use host audio devices"),
                ]
                .spacing(18);
                #[cfg(feature = "asr")]
                let controls = controls.push(asr_controls);
                controls
            })
            .width(Length::Fill)
            .height(Length::Shrink)
            .padding(24),
        )
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }
}

impl Drop for State {
    fn drop(&mut self) {
        #[cfg(feature = "asr")]
        self.transcription_job.cancel();
        #[cfg(feature = "asr")]
        self.model_check_job.cancel();
        let _ = self.commands.send(Command::Shutdown);
    }
}

#[cfg(feature = "asr")]
fn transcribe_task(
    path: PathBuf,
    model: AsrModel,
    translate_to_english: bool,
    job: Job,
) -> Task<Message> {
    Task::perform(
        async move {
            let worker_path = path.clone();
            let result = thread::spawn(move || {
                if translate_to_english {
                    crate::translate_speech_to_english_file(&worker_path, model.parameter(), &job)
                } else {
                    crate::transcribe_file(&worker_path, model.parameter(), &job)
                }
                .map_err(|error| error.message)
            })
            .join()
            .unwrap_or_else(|_| Err("Offline transcription worker stopped unexpectedly".into()));
            (path, translate_to_english, result)
        },
        |(path, translated, result)| Message::TranscriptionFinished(path, translated, result),
    )
}

#[cfg(feature = "asr")]
fn model_status_task(model: AsrModel, job: Job) -> Task<Message> {
    Task::perform(
        async move {
            let result = thread::spawn(move || {
                crate::verify_private_model_pack(model.parameter(), &job)
                    .map_err(|error| error.message)
            })
            .join()
            .unwrap_or_else(|_| Err("Model verification worker stopped unexpectedly".into()));
            (model, result)
        },
        |(model, result)| Message::ModelStatusChecked(model, result),
    )
}

#[derive(Debug, Clone)]
enum Message {
    OpenDialog,
    FilePicked(Option<PathBuf>),
    RecordDialog,
    RecordPath(Option<PathBuf>),
    RefreshInputs,
    InputSelected(String),
    StopRecording,
    CancelRecording,
    Transport,
    Stop,
    Scrub(f32),
    SeekReleased,
    WaveformSeek(u64),
    OpenTranscript,
    TranscriptPicked(Option<PathBuf>),
    LoadLinkedAudio,
    SaveProject,
    ProjectSavePath(Option<PathBuf>),
    SelectCue(usize),
    CuePagePrevious,
    CuePageNext,
    CueStartChanged(String),
    CueEndChanged(String),
    TranscriptOffsetChanged(String),
    ApplyTranscriptOffset,
    CueTextAction(text_editor::Action),
    ApplyCueText,
    FindTextChanged(String),
    ReplacementTextChanged(String),
    ReplaceSelected,
    ReplaceAll,
    ApplyCueTiming,
    UndoCueTiming,
    RedoCueTiming,
    #[cfg(feature = "asr")]
    AsrModelChanged(AsrModel),
    #[cfg(feature = "asr")]
    ModelStatusChecked(AsrModel, std::result::Result<String, String>),
    #[cfg(feature = "asr")]
    Transcribe,
    #[cfg(feature = "asr")]
    TranslateToEnglish,
    #[cfg(feature = "asr")]
    CancelTranscription,
    #[cfg(feature = "asr")]
    TranscriptionFinished(PathBuf, bool, std::result::Result<Vec<Segment>, String>),
    Tick,
}

fn load_transcript(path: &Path) -> Result<DecodedWorkspaceProject, String> {
    let bytes = read_bounded(path, 16 * 1024 * 1024).map_err(|error| error.message)?;
    let is_json = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    decode_workspace_project(&bytes, is_json)
}

fn save_project(
    path: &Path,
    cues: &[Segment],
    audio_path: Option<&Path>,
    transcript_offset_ms: i64,
) -> Result<(), String> {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err("Choose a .json project path".into());
    }
    let relative_audio = audio_path
        .map(|audio| relative_media_path(path, audio))
        .transpose()?;
    let bytes = encode_workspace_project(cues, relative_audio.as_deref(), transcript_offset_ms)?;
    write_bytes(path, &bytes).map_err(|error| error.message)
}

fn relative_media_path(project_path: &Path, audio_path: &Path) -> Result<String, String> {
    let project_dir = project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let project_dir = std::fs::canonicalize(project_dir)
        .map_err(|error| format!("Could not resolve project directory: {error}"))?;
    let audio_path = std::fs::canonicalize(audio_path)
        .map_err(|error| format!("Could not resolve linked audio: {error}"))?;
    let from = project_dir.components().collect::<Vec<_>>();
    let to = audio_path.components().collect::<Vec<_>>();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return Err("Project and audio are on different filesystem roots; choose a project directory on the same volume".into());
    }
    let mut parts = Vec::new();
    for component in &from[common..] {
        if matches!(component, std::path::Component::Normal(_)) {
            parts.push("..".to_owned());
        }
    }
    for component in &to[common..] {
        if let std::path::Component::Normal(name) = component {
            let name = name
                .to_str()
                .ok_or_else(|| "Linked audio path contains non-Unicode characters and cannot be stored portably".to_owned())?;
            parts.push(name.to_owned());
        }
    }
    if parts.is_empty() {
        return Err("Could not form a relative path to the audio file".into());
    }
    Ok(parts.join("/"))
}

fn update_snapshot(snapshot: &Arc<Mutex<Snapshot>>, update: impl FnOnce(&mut Snapshot)) {
    update(&mut lock_snapshot(snapshot));
}

fn lock_snapshot(snapshot: &Mutex<Snapshot>) -> MutexGuard<'_, Snapshot> {
    snapshot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn audio_worker(receiver: mpsc::Receiver<Command>, snapshot: Arc<Mutex<Snapshot>>) {
    let mut clip: Option<PlaybackClip> = None;
    let mut session: Option<Session> = None;
    let mut recording: Option<CaptureSession> = None;
    let mut input_devices = Vec::<InputDevice>::new();
    let mut selected_input = None;
    let mut position_ms = 0u64;
    loop {
        match receiver.recv_timeout(Duration::from_millis(80)) {
            Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Command::Open(path)) => {
                session = None;
                clip = None;
                position_ms = 0;
                update_snapshot(&snapshot, |view| {
                    view.loading = true;
                    view.loaded = false;
                    view.playing = false;
                    view.paused = false;
                    view.position_ms = 0;
                    view.message = "Reading and validating audio…".into();
                    view.file_name.clear();
                    view.audio_path = None;
                    view.waveform.clear();
                    view.duration_ms = 0;
                });
                match load_playback_clip(&path, &Job::new()) {
                    Ok(loaded) => {
                        let name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("Audio file")
                            .to_owned();
                        update_snapshot(&snapshot, |view| {
                            view.file_name = name;
                            view.audio_path = Some(path.clone());
                            view.waveform = loaded.waveform.clone();
                            view.duration_ms = loaded.duration_ms;
                            view.position_ms = 0;
                            view.loaded = true;
                            view.loading = false;
                            view.message = "Ready".into();
                        });
                        clip = Some(loaded);
                    }
                    Err(error) => update_snapshot(&snapshot, |view| {
                        view.loading = false;
                        view.audio_path = None;
                        view.message = format!("Could not open audio: {}", error.message);
                    }),
                }
            }
            Ok(Command::Play) => {
                if let Some(loaded) = clip.as_ref() {
                    if position_ms >= loaded.duration_ms {
                        position_ms = 0;
                        update_snapshot(&snapshot, |view| view.position_ms = 0);
                    }
                    match start_session(loaded, position_ms) {
                        Ok(playback) => {
                            session = Some(playback);
                            update_snapshot(&snapshot, |view| {
                                view.playing = true;
                                view.paused = false;
                                view.message = "Playing".into();
                            });
                        }
                        Err(error) => update_snapshot(&snapshot, |view| {
                            view.message = format!("Playback failed: {error}");
                        }),
                    }
                }
            }
            Ok(Command::Pause) => {
                if let Some(playback) = &session {
                    playback.sink.pause();
                    update_snapshot(&snapshot, |view| {
                        view.paused = true;
                        view.message = "Paused".into();
                    });
                }
            }
            Ok(Command::Resume) => {
                if let Some(playback) = &session {
                    playback.sink.play();
                    update_snapshot(&snapshot, |view| {
                        view.paused = false;
                        view.message = "Playing".into();
                    });
                }
            }
            Ok(Command::Stop) => {
                session = None;
                position_ms = 0;
                update_snapshot(&snapshot, |view| {
                    view.playing = false;
                    view.paused = false;
                    view.position_ms = 0;
                    view.message = if view.loaded {
                        "Stopped"
                    } else {
                        "No audio loaded"
                    }
                    .into();
                });
            }
            Ok(Command::RefreshInputs) => match enumerate_inputs() {
                Ok(devices) => {
                    if !devices
                        .iter()
                        .any(|device| Some(device.index) == selected_input)
                    {
                        selected_input = devices.first().map(|device| device.index);
                    }
                    input_devices = devices;
                    update_snapshot(&snapshot, |view| {
                        view.input_devices = input_devices
                            .iter()
                            .map(|device| device.label.clone())
                            .collect();
                        view.selected_input = selected_input.and_then(|index| {
                            input_devices
                                .iter()
                                .find(|device| device.index == index)
                                .map(|device| device.label.clone())
                        });
                        if input_devices.is_empty() {
                            view.message = "No audio input devices are available".into();
                        }
                    });
                }
                Err(error) => update_snapshot(&snapshot, |view| {
                    view.message = error;
                    view.input_devices.clear();
                    view.selected_input = None;
                }),
            },
            Ok(Command::SelectInput(index)) => {
                if recording.is_some() {
                    update_snapshot(&snapshot, |view| {
                        view.message = "Stop recording before changing the input device".into();
                    });
                } else if input_devices.iter().any(|device| device.index == index) {
                    selected_input = Some(index);
                    update_snapshot(&snapshot, |view| {
                        view.selected_input = input_devices
                            .iter()
                            .find(|device| device.index == index)
                            .map(|device| device.label.clone());
                        view.message = "Input device selected".into();
                    });
                }
            }
            Ok(Command::StartRecording(path)) => {
                if recording.is_some() {
                    update_snapshot(&snapshot, |view| {
                        view.message = "A recording is already active".into();
                    });
                } else {
                    match start_capture(&path, selected_input) {
                        Ok(capture) => {
                            recording = Some(capture);
                            update_snapshot(&snapshot, |view| {
                                view.recording = true;
                                view.recording_ms = 0;
                                view.message = format!("Recording to {}", path.display());
                            });
                        }
                        Err(error) => update_snapshot(&snapshot, |view| {
                            view.recording = false;
                            view.message = format!("Could not start recording: {error}");
                        }),
                    }
                }
            }
            Ok(Command::StopRecording) => finish_capture(&mut recording, &snapshot, None),
            Ok(Command::CancelRecording) => cancel_capture(&mut recording, &snapshot),
            Ok(Command::Seek(target_ms)) => {
                position_ms = clip
                    .as_ref()
                    .map(|loaded| target_ms.min(loaded.duration_ms))
                    .unwrap_or(0);
                let seek_error = if let Some(playback) = session.as_ref() {
                    playback
                        .sink
                        .try_seek(Duration::from_millis(position_ms))
                        .err()
                        .map(|error| {
                            position_ms = playback.sink.get_pos().as_millis() as u64;
                            error.to_string()
                        })
                } else {
                    None
                };
                update_snapshot(&snapshot, |view| {
                    view.position_ms = position_ms;
                    if let Some(error) = seek_error {
                        view.message = format!("Could not seek: {error}");
                    }
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Some(capture) = recording.as_mut() {
            if let Err(error) = capture.drain() {
                finish_capture(&mut recording, &snapshot, Some(error));
            } else {
                let progress = capture.duration_ms();
                update_snapshot(&snapshot, |view| view.recording_ms = progress);
            }
        }

        let finished = session
            .as_ref()
            .is_some_and(|playback| playback.sink.empty());
        if finished {
            session = None;
            position_ms = clip.as_ref().map_or(0, |loaded| loaded.duration_ms);
            update_snapshot(&snapshot, |view| {
                view.position_ms = position_ms;
                view.playing = false;
                view.paused = false;
                view.message = "Playback finished".into();
            });
        } else if let Some(playback) = &session {
            position_ms = playback
                .sink
                .get_pos()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            update_snapshot(&snapshot, |view| view.position_ms = position_ms);
        }
    }
    finish_capture(
        &mut recording,
        &snapshot,
        Some("Recording stopped when the workspace closed".into()),
    );
}

fn finish_capture(
    recording: &mut Option<CaptureSession>,
    snapshot: &Arc<Mutex<Snapshot>>,
    reason: Option<String>,
) {
    let Some(capture) = recording.take() else {
        return;
    };
    match capture.finish() {
        Ok(finalized) => update_snapshot(snapshot, |view| {
            view.recording = false;
            view.recording_ms = 0;
            let elapsed = if finalized.frames == 0 {
                "empty WAV".into()
            } else {
                format!(
                    "{:.2} seconds",
                    finalized.frames as f64 / f64::from(finalized.sample_rate)
                )
            };
            let partial_reason = match (reason, finalized.partial_reason) {
                (Some(first), Some(second)) => Some(format!("{first}; {second}")),
                (Some(reason), None) | (None, Some(reason)) => Some(reason),
                (None, None) => None,
            };
            view.message = match partial_reason {
                Some(reason) => format!(
                    "Saved partial recording ({elapsed}) to {}: {reason}",
                    finalized.path.display()
                ),
                None => format!(
                    "Saved recording ({elapsed}) to {}",
                    finalized.path.display()
                ),
            };
        }),
        Err(error) => update_snapshot(snapshot, |view| {
            view.recording = false;
            view.recording_ms = 0;
            view.message = format!("Could not finalize recording: {error}");
        }),
    }
}

fn cancel_capture(recording: &mut Option<CaptureSession>, snapshot: &Arc<Mutex<Snapshot>>) {
    let Some(capture) = recording.take() else {
        return;
    };
    match capture.cancel() {
        Ok(path) => update_snapshot(snapshot, |view| {
            view.recording = false;
            view.recording_ms = 0;
            view.message = format!("Discarded new recording at {path}");
        }),
        Err(error) => update_snapshot(snapshot, |view| {
            view.recording = false;
            view.recording_ms = 0;
            view.message = format!("Could not discard recording: {error}");
        }),
    }
}

fn start_session(clip: &PlaybackClip, seek_ms: u64) -> Result<Session, String> {
    let (stream, handle): (OutputStream, OutputStreamHandle) =
        OutputStream::try_default().map_err(|error| error.to_string())?;
    let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
    let decoder = open_audio_decoder(Arc::clone(&clip.bytes), &Job::new())
        .map_err(|error| error.to_string())?;
    if decoder.sample_rate() != clip.sample_rate {
        return Err("WAV sample rate changed while opening the decoder".into());
    }
    sink.append(decoder);
    if seek_ms > 0 {
        sink.try_seek(Duration::from_millis(seek_ms))
            .map_err(|error| error.to_string())?;
    }
    Ok(Session {
        sink,
        _stream: stream,
    })
}
