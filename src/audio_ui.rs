#![forbid(unsafe_code)]

use crate::recording::{enumerate_inputs, start_capture, CaptureSession, InputDevice};
use crate::{
    audio_model::{
        active_cue, clamp_seek, decode_workspace_project, encode_workspace_project, format_time,
        transport_action, waveform_seek, CueHistory, DecodedWorkspaceProject, TransportAction,
    },
    load_playback_clip, open_audio_decoder, parse_stamp, stamp, PlaybackClip, Segment,
};
use iced::{
    widget::{
        button, canvas, column, container, pick_list, row, scrollable, slider, text, text_input,
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
    linked_audio: Option<PathBuf>,
    notice: String,
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
                linked_audio: None,
                notice: String::new(),
            },
            Task::none(),
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
                    self.cues = CueHistory::new(project.segments);
                    self.linked_audio = project.audio_path.map(|media| {
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
                match save_project(&path, self.cues.cues(), self.view.audio_path.as_deref()) {
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
                    let (start, end) = (cue.start, cue.end);
                    self.notice.clear();
                    self.selected_cue = Some(index);
                    self.page_start = index / CUES_PER_PAGE * CUES_PER_PAGE;
                    self.cue_start_text = stamp(start);
                    self.cue_end_text = stamp(end);
                    self.send(Command::Seek(start));
                }
            }
            Message::CuePagePrevious => {
                self.page_start = self.page_start.saturating_sub(CUES_PER_PAGE);
            }
            Message::CuePageNext => {
                if self.page_start + CUES_PER_PAGE < self.cues.cues().len() {
                    self.page_start += CUES_PER_PAGE;
                }
            }
            Message::CueStartChanged(value) => self.cue_start_text = value,
            Message::CueEndChanged(value) => self.cue_end_text = value,
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
                    "Cue timing edit undone.".into()
                } else {
                    "No cue timing edit to undo.".into()
                };
            }
            Message::RedoCueTiming => {
                self.notice = if self.cues.redo() {
                    self.refresh_selected_timing();
                    "Cue timing edit redone.".into()
                } else {
                    "No cue timing edit to redo.".into()
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
                if let Some(active) = active_cue(self.cues.cues(), self.view.position_ms) {
                    let page_start = active / CUES_PER_PAGE * CUES_PER_PAGE;
                    if page_start != self.page_start {
                        self.page_start = page_start;
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
        }
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
        let active = active_cue(cues, self.view.position_ms);
        let cue_rows = cues
            .iter()
            .enumerate()
            .skip(self.page_start)
            .take(CUES_PER_PAGE)
            .map(|(index, cue)| {
                let marker = if active == Some(index) {
                    "▶"
                } else if self.selected_cue == Some(index) {
                    "●"
                } else {
                    " "
                };
                let cue_text = cue.text.chars().take(120).collect::<String>();
                button(text(format!(
                    "{marker} {:>4}  {}–{}  {cue_text}",
                    index + 1,
                    format_time(cue.start),
                    format_time(cue.end),
                )))
                .width(Length::Fill)
                .on_press(Message::SelectCue(index))
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();
        let cue_list = scrollable(column(cue_rows).spacing(4)).height(Length::Fixed(240.0));
        let visible_end = (self.page_start + CUES_PER_PAGE).min(cues.len());
        let visible_range = if cues.is_empty() {
            "No transcript loaded".into()
        } else {
            format!(
                "Cues {}–{visible_end} of {}",
                self.page_start + 1,
                cues.len()
            )
        };
        scrollable(
            container(
                column![
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
                        button("Undo")
                            .on_press_maybe(self.cues.can_undo().then_some(Message::UndoCueTiming)),
                        button("Redo")
                            .on_press_maybe(self.cues.can_redo().then_some(Message::RedoCueTiming)),
                        button("‹ Cues").on_press_maybe(
                            (self.page_start > 0).then_some(Message::CuePagePrevious)
                        ),
                        text(visible_range),
                        button("Cues ›").on_press_maybe(
                            (self.page_start + CUES_PER_PAGE < cues.len())
                                .then_some(Message::CuePageNext)
                        ),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .wrap(),
                    cue_list,
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
                .spacing(18),
            )
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
        let _ = self.commands.send(Command::Shutdown);
    }
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
    ApplyCueTiming,
    UndoCueTiming,
    RedoCueTiming,
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

fn save_project(path: &Path, cues: &[Segment], audio_path: Option<&Path>) -> Result<(), String> {
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
    let bytes = encode_workspace_project(cues, relative_audio.as_deref())?;
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
