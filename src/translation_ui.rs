#![forbid(unsafe_code)]

//! Native text-translation workspace for the separately installed model pack.

use iced::{
    widget::{button, column, container, row, text, text_editor},
    Element, Length, Task,
};
use localwright_core::Job;
use std::{
    path::{Path, PathBuf},
    thread,
};

pub fn run() -> iced::Result {
    iced::application(
        "Speech Studio — Text translation",
        State::update,
        State::view,
    )
    .run_with(State::new)
}

struct State {
    source: text_editor::Content,
    translated: text_editor::Content,
    busy: bool,
    job: Job,
    notice: String,
}

impl State {
    fn new() -> Self {
        Self {
            source: text_editor::Content::new(),
            translated: text_editor::Content::new(),
            busy: false,
            job: Job::new(),
            notice: "English → French · offline Marian model pack required".into(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SourceAction(action) => self.source.perform(action),
            Message::OutputAction(action) => self.translated.perform(action),
            Message::CheckPack => {
                self.notice = "Checking pinned model and tokenizer files…".into();
                return Task::perform(
                    async move {
                        thread::spawn(|| {
                            crate::translation::verify_private_pack(&Job::new())
                                .map_err(|error| error.message)
                        })
                        .join()
                        .unwrap_or_else(|_| {
                            Err("Model verification worker stopped unexpectedly".into())
                        })
                    },
                    Message::PackChecked,
                );
            }
            Message::PackChecked(result) => {
                self.notice =
                    result.unwrap_or_else(|error| format!("Model pack unavailable: {error}"));
            }
            Message::Translate => {
                if self.busy {
                    self.notice = "Translation is already running.".into();
                } else {
                    let source = self.source.text();
                    if source.trim().is_empty() {
                        self.notice = "Enter English text to translate.".into();
                    } else if source.len() > 8 * 1024 {
                        self.notice = "Translation input is limited to 8 KiB.".into();
                    } else {
                        self.busy = true;
                        self.job = Job::new();
                        let job = self.job.clone();
                        self.notice =
                            "Translating offline. The verified model loads in the background…"
                                .into();
                        return Task::perform(
                            async move {
                                thread::spawn(move || {
                                    crate::translation::translate_english_to_french(&source, &job)
                                        .map_err(|error| error.message)
                                })
                                .join()
                                .unwrap_or_else(|_| {
                                    Err("Translation worker stopped unexpectedly".into())
                                })
                            },
                            Message::TranslationFinished,
                        );
                    }
                }
            }
            Message::Cancel => {
                self.job.cancel();
                self.notice = "Cancellation requested; the decoder checks between tokens.".into();
            }
            Message::TranslationFinished(result) => {
                self.busy = false;
                match result {
                    Ok(text) => {
                        self.translated = text_editor::Content::with_text(&text);
                        self.notice = format!(
                            "Translation complete ({} characters). Review it before use.",
                            text.chars().count()
                        );
                    }
                    Err(error) => self.notice = format!("Translation failed: {error}"),
                }
            }
            Message::Export => {
                let text = self.translated.text();
                if text.trim().is_empty() {
                    self.notice = "There is no translated text to export.".into();
                } else {
                    return Task::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_title("Save translated French text")
                                .add_filter("Plain text", &["txt"])
                                .save_file()
                                .await
                                .map(|file| (text, file.path().to_path_buf()))
                        },
                        Message::ExportPicked,
                    );
                }
            }
            Message::Copy => {
                let translated = self.translated.text();
                if translated.trim().is_empty() {
                    self.notice = "There is no translated text to copy.".into();
                } else {
                    return iced::clipboard::write(translated);
                }
            }
            Message::ExportPicked(Some((text, path))) => {
                return Task::perform(
                    async move {
                        thread::spawn(move || write_new_text(&path, &text))
                            .join()
                            .unwrap_or_else(|_| Err("Export worker stopped unexpectedly".into()))
                    },
                    Message::ExportFinished,
                );
            }
            Message::ExportPicked(None) => {}
            Message::ExportFinished(result) => {
                self.notice =
                    result.unwrap_or_else(|error| format!("Could not export text: {error}"));
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let translate_button = if self.busy {
            "Cancel translation"
        } else {
            "Translate offline"
        };
        let action = if self.busy {
            Message::Cancel
        } else {
            Message::Translate
        };
        let content = column![
            text("Text translation").size(26),
            text("English → French · OPUS-MT Marian · CPU inference · no network access"),
            row![
                button("Verify offline model pack").on_press(Message::CheckPack),
                button(translate_button).on_press(action),
                button("Copy French text").on_press(Message::Copy),
                button("Export French text…").on_press(Message::Export),
            ]
            .spacing(10)
            .wrap(),
            text(&self.notice),
            text("English source (maximum 8 KiB)"),
            text_editor(&self.source)
                .on_action(Message::SourceAction)
                .height(Length::FillPortion(1)),
            text("French translation (editable; review before use)"),
            text_editor(&self.translated)
                .on_action(Message::OutputAction)
                .height(Length::FillPortion(1)),
        ]
        .spacing(12)
        .padding(24)
        .height(Length::Fill);
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn write_new_text(path: &Path, text: &str) -> Result<(), String> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
    {
        return Err("Choose a .txt output".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "The selected output already exists; choose a new path".to_owned()
            } else {
                format!("Could not create output: {error}")
            }
        })?;
    use std::io::Write;
    if let Err(error) = file.write_all(text.as_bytes()) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(format!("Could not write output: {error}"));
    }
    Ok(())
}

enum Message {
    SourceAction(text_editor::Action),
    OutputAction(text_editor::Action),
    CheckPack,
    PackChecked(Result<String, String>),
    Translate,
    Cancel,
    TranslationFinished(Result<String, String>),
    Export,
    Copy,
    ExportPicked(Option<(String, PathBuf)>),
    ExportFinished(Result<(), String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_export_is_exclusive_and_keeps_existing_data() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("french.txt");
        write_new_text(&path, "Bonjour").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Bonjour");
        assert!(write_new_text(&path, "perdu").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Bonjour");
    }

    #[test]
    fn translation_export_requires_text_extension() {
        let directory = tempfile::tempdir().unwrap();
        assert!(write_new_text(&directory.path().join("out.csv"), "bonjour").is_err());
    }
}
