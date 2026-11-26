# Verification evidence

The integration run used Rust 1.88.0 on Linux x86_64 with Localwright pinned to the revision in `Cargo.lock`. Formatting, locked all-target checking, default and headless tests, strict Clippy, and CLI help passed.

The WAV and SRT tests passed. Python wave confirmed the CLI duration and sample count, and the subtitle export preserved the fixture text. The optional ASR adapter compiled on Linux. Model inference, accuracy and cancellation during inference remain open.

The GitHub workflow defines five platform checks. Device tests and installer verification remain open.
