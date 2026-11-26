# Architecture

Speech Studio: Local speech and transcript workbench.

Localwright core contains serde-based domain contracts, request/response, capability and engine descriptors, job cancellation, bounded reads, no-clobber output and settings. It has no Iced dependency. Localwright UI is an optional dependency of the studios and provides a generic native operation workbench. It dispatches a single job to a background thread and receives the structured outcome through a channel. The same handler runs through the CLI. Studio processing lives in its own crate. No local server, system engine lookup or runtime network client is intentionally implemented.

Source contracts version 0.1.0-alpha.1; pre-1.0 API is unstable. Use exact shared revision after import. New breaking changes require migration notes and all four consumer checks. Candidate Rust minimum 1.85 (untested), toolchain 1.88.0 (untested). No promise of ABI stability. Current settings schema 1. Request and outcome are internal alpha interchange, not stable project file formats.

Limitations: No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Explicit local model path and expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.
