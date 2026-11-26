# Verification evidence

The integration run used Rust 1.88.0 on Linux x86_64 with Localwright pinned to the revision in `Cargo.lock`. Formatting, locked all-target checking, default and headless tests, strict Clippy, and CLI help passed.

The WAV and SRT tests passed. Python wave confirmed the CLI duration and sample count, and the subtitle export preserved the fixture text. WebVTT fixture checks confirmed timing and literal Unicode cue text.

The optional CPU ASR build ran on Linux x86_64 with a local [Tiny English model](https://huggingface.co/ggerganov/whisper.cpp/blob/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny.en.bin) and the upstream [JFK WAV sample](https://github.com/ggml-org/whisper.cpp/blob/79b11911d9d38c4be2fc7dc8b4efeff3324c090f/samples/jfk.wav). The expected model SHA-256 was `921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f`. The sample produced two SRT segments. With socket creation blocked, no socket or connection calls were observed. Native decoder logs were suppressed and the bundled silence fixture returned an error. This covers one English sample. Accuracy across languages, cancellation during inference and model redistribution remain open.

The GitHub workflow defines five platform checks. Device tests and installer verification remain open.
