#![forbid(unsafe_code)]

use crate::{
    audio_model::{clamp_seek, format_time, transport_action, TransportAction},
    load_playback_clip, PlaybackClip,
};
use iced::{
    widget::{button, canvas, column, container, row, slider, text},
    Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme,
};
use localwright_core::Job;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{
    io::Cursor,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

struct Waveform<'a> {
    peaks: &'a [f32],
    position_ms: u64,
    duration_ms: u64,
}

impl<Message> canvas::Program<Message> for Waveform<'_> {
    type State = ();

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
    waveform: Vec<f32>,
    duration_ms: u64,
    position_ms: u64,
    loaded: bool,
    loading: bool,
    playing: bool,
    paused: bool,
    message: String,
}

enum Command {
    Open(PathBuf),
    Play,
    Pause,
    Resume,
    Stop,
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
}

impl State {
    fn new() -> (Self, Task<Message>) {
        let snapshot = Arc::new(Mutex::new(Snapshot {
            message: "Open a PCM16 WAV file to begin.".into(),
            ..Snapshot::default()
        }));
        let (commands, receiver) = mpsc::channel();
        let worker_snapshot = Arc::clone(&snapshot);
        if let Err(error) = thread::Builder::new()
            .name("speech-audio-player".into())
            .spawn(move || audio_worker(receiver, worker_snapshot))
        {
            lock_snapshot(&snapshot).message = format!("Could not start audio worker: {error}");
        }
        let view = lock_snapshot(&snapshot).clone();
        (
            Self {
                commands,
                snapshot,
                view,
                seek_value_ms: None,
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
                            .set_title("Open PCM16 WAV audio")
                            .add_filter("PCM16 WAV", &["wav"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::FilePicked,
                );
            }
            Message::FilePicked(Some(path)) => {
                self.seek_value_ms = None;
                if self.commands.send(Command::Open(path)).is_ok() {
                    self.view.loading = true;
                    self.view.message = "Reading and validating audio…".into();
                } else {
                    self.view.message = "The audio worker has stopped.".into();
                }
            }
            Message::FilePicked(None) => {}
            Message::Transport => {
                match transport_action(self.view.loaded, self.view.playing, self.view.paused) {
                    Some(TransportAction::Play) => self.send(Command::Play),
                    Some(TransportAction::Pause) => self.send(Command::Pause),
                    Some(TransportAction::Resume) => self.send(Command::Resume),
                    None => {}
                }
            }
            Message::Stop => self.send(Command::Stop),
            Message::Scrub(value) => self.seek_value_ms = Some(value),
            Message::SeekReleased => {
                if let Some(value) = self.seek_value_ms.take() {
                    self.send(Command::Seek(clamp_seek(value, self.view.duration_ms)));
                }
            }
            Message::Tick(_) => {
                self.view = lock_snapshot(&self.snapshot).clone();
            }
        }
        Task::none()
    }

    fn send(&mut self, command: Command) {
        if self.commands.send(command).is_err() {
            self.view.message = "The audio worker has stopped.".into();
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
        container(
            column![
                row![
                    text("Speech Studio audio workspace").size(24),
                    button("Open WAV…").on_press(Message::OpenDialog),
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
                    button(action_label).on_press_maybe(action.map(|_| Message::Transport)),
                    button("Stop").on_press_maybe(self.view.loaded.then_some(Message::Stop)),
                    text(if self.view.loading {
                        "Loading audio…".to_owned()
                    } else {
                        self.view.message.clone()
                    })
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
                .wrap(),
                text("WAV only · local processing · physical audio device required for playback"),
            ]
            .spacing(18),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(Message::Tick)
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
    Transport,
    Stop,
    Scrub(f32),
    SeekReleased,
    Tick(iced::time::Instant),
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
}

fn start_session(clip: &PlaybackClip, seek_ms: u64) -> Result<Session, String> {
    let (stream, handle): (OutputStream, OutputStreamHandle) =
        OutputStream::try_default().map_err(|error| error.to_string())?;
    let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
    let decoder =
        Decoder::new(Cursor::new(Arc::clone(&clip.bytes))).map_err(|error| error.to_string())?;
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
