# Status | 0.1.0-alpha.1

Alpha source is on GitHub. The initial Rust integration checks ran on Linux x86_64.

| Operation | Scope | Status |
|---|---|---|
| audio-info | 16 kHz mono signed PCM16 WAV | Linux fixture-tested |
| open-srt | UTF-8 SRT subset; editable draft | Linux fixture-tested |
| export-srt | Validated SRT output | Linux fixture-tested |
| export-txt | Plain transcript text | Linux fixture-tested |
| transcribe | Optional CPU whisper-rs adapter, no bundled model | Linux compile checked, inference unverified |

## Known gaps

No redistributed ASR model or reference speech. Native inference is optional and disabled by default. Explicit local model path and expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

Hosted matrix, device, and packaging verification remain open. See [verification evidence](VERIFICATION.md) and [dependency review](DEPENDENCIES.md).
