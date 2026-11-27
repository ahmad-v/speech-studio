# Status | 0.1.0-alpha.1

Alpha source is on GitHub. The initial Rust integration checks ran on Linux x86_64.

| Operation | Scope | Status |
|---|---|---|
| scale-timing | Scale cue times from zero | Linux fixture-tested |
| trim-cues | Crop cues to a time interval | Linux fixture-tested |
| split-cue | Split a cue at an exact millisecond | Linux fixture-tested |
| merge-cues | Join consecutive cue text and timing | Linux fixture-tested |
| delete-cue | Remove and renumber a selected cue | Linux fixture-tested |
| shift-timing | Shift all subtitle cues by signed milliseconds | Linux fixture and CLI-tested |
| waveform-svg | Export a peak SVG from mono 16 kHz PCM16 WAV | Linux fixture and CLI-tested |
| open-vtt | Import plain-text WebVTT cues into an SRT draft | Linux fixture-tested |
| prepare-wav | PCM16 WAV at 16, 32, 44.1 or 48 kHz to mono 16 kHz | Linux fixture-tested |
| audio-info | 16 kHz mono signed PCM16 WAV | Linux fixture-tested |
| replace-text | Exact transcript find and replace within cues | Linux fixture and CLI-tested |
| open-srt | UTF-8 SRT subset; editable draft | Linux fixture-tested |
| export-srt | Validated SRT output | Linux fixture-tested |
| export-vtt | Plain-text WebVTT from SRT input or draft | Linux fixture-tested |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | Linux fixture-tested |
| transcribe | Optional CPU whisper-rs adapter, optional private Tiny English pack or an explicit model path | Linux x64 app-relative model and export smoke-tested |

## Known gaps

The standard edition omits the optional private ASR pack. Native inference is optional and disabled by default. Private Tiny English pack or explicit local model path with expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. Playback, waveform, wider media decoding, language accuracy, WER, translation, diarization and TTS remain open. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

Hosted matrix, device, and packaging verification remain open. See [verification evidence](VERIFICATION.md) and [dependency review](DEPENDENCIES.md).
