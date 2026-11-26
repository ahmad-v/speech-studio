# Status | 0.1.0-alpha.1

M0 complete as source planning; partial M1 implementation, not verified. All five repos created locally as source trees only. No remote resources created.

| Operation | Scope | Status |
|---|---|---|
| audio-info | 16 kHz mono signed PCM16 WAV | implemented-unverified |
| open-srt | UTF-8 SRT subset; editable draft | implemented-unverified |
| export-srt | Validated SRT output | implemented-unverified |
| export-txt | Plain transcript text | implemented-unverified |
| transcribe | Optional CPU whisper-rs adapter, no bundled model | blocked |

## Known gaps

No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Explicit local model path and expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

Cargo and rustc absent; registry and distribution DNS failed. No lockfile, compilation, rustfmt, clippy, Rust tests, GUI/device tests, package builds or security audit performed. All five platform builds blocked pending CI. See MASTER_SPECIFICATION.md for authoritative acceptance gates.
