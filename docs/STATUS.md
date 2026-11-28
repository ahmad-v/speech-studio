# Status | 0.1.0-alpha.1

Alpha source is on GitHub. The initial Rust integration checks ran on Linux x86_64.

| Operation | Scope | Status |
|---|---|---|
| scale-timing | Scale cue times from zero | Linux fixture-tested |
| trim-cues | Crop cues to a time interval | Linux fixture-tested |
| split-cue | Split a cue at an exact millisecond | Linux fixture-tested |
| merge-cues | Join consecutive cue text and timing | Linux fixture-tested |
| delete-cue | Remove and renumber a selected cue | Linux fixture-tested |
| replace-cue-text | Replace one cue's text while retaining timing | Linux cue-edit fixture-tested; hosted matrix pending |
| insert-cue | Insert a cue at a requested index with validated timing | Linux cue-edit fixture-tested; hosted matrix pending |
| set-cue-timing | Replace one cue's interval with order validation | Linux cue-edit fixture-tested; hosted matrix pending |
| shift-cue-range | Shift an inclusive cue range with bounds checks | Linux cue-edit fixture-tested; hosted matrix pending |
| remove-duplicate-cues | Remove exact duplicate records, retaining first occurrence | Linux cue-edit fixture-tested; hosted matrix pending |
| normalize-cue-whitespace | Collapse whitespace per line and preserve line breaks | Linux cue-edit fixture-tested; hosted matrix pending |
| wrap-cue-text | Wrap at word boundaries using grapheme width | Linux cue-edit fixture-tested; hosted matrix pending |
| join-cue-lines | Join cue lines and collapse whitespace | Linux cue-edit fixture-tested; hosted matrix pending |
| swap-cue-text | Exchange cue text while retaining timing | Linux cue-edit fixture-tested; hosted matrix pending |
| replace-text-range | Replace literal text in an inclusive cue range | Linux cue-edit fixture-tested; hosted matrix pending |
| shift-timing | Shift all subtitle cues by signed milliseconds | Linux fixture and CLI-tested |
| waveform-svg | Export a peak SVG from mono 16 kHz PCM16 WAV | Linux fixture and CLI-tested |
| open-vtt | Import plain-text WebVTT cues into an SRT draft | Linux fixture-tested |
| open-lrc | Import timed LRC lyric lines into the SRT draft | Linux fixture-tested |
| prepare-wav | PCM16 WAV at 16, 32, 44.1 or 48 kHz to mono 16 kHz | Linux fixture-tested |
| audio-info | 16 kHz mono signed PCM16 WAV | Linux fixture-tested |
| play-wav | Native Iced audio workspace: bounded local WAV/FLAC/MP3/OGG/AAC/AIFF decoding and playback, device enumeration/selection, bounded PCM16 WAV recording, live elapsed status and stop/finalize, waveform seek, synchronized transcript and cue timing undo/redo; v2 workspace JSON stores a relative linked-audio path and retains v1 compatibility | Audio-project model tests passed; GUI build awaits hosted verification; device enumeration, playback and recording are not hardware-verified |
| detect-voice-activity | Adaptive RMS and hysteresis intervals from 16 kHz mono PCM16 WAV, JSON report | Linux silence, synthetic activity and operation-output fixtures tested; energy heuristic only |
| analyze-transcript | Local language estimate, Unicode linguistic counts and subtitle-time rate estimate, JSON report | Linux English, Russian, short-text, overlap and operation-output fixtures tested; estimates only |
| replace-text | Exact transcript find and replace within cues | Linux fixture and CLI-tested |
| open-srt | UTF-8 SRT subset; editable draft | Linux fixture-tested |
| export-srt | Validated SRT output | Linux fixture-tested |
| export-vtt | Plain-text WebVTT from SRT input or draft | Linux fixture-tested |
| export-lrc | Export cue start times and text to LRC | Linux fixture-tested |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | Linux fixture-tested |
| transcribe | Optional CPU whisper-rs adapter, private English or multilingual Tiny pack, or an explicit model path; multilingual pack pins its source bytes and uses language auto-detection | Existing English model app-relative smoke-tested. Multilingual artifact bytes verified against pinned Hugging Face LFS SHA-256; Check and Linux Integration passed at `4412c9f`, while the all-target optional-ASR matrix and pack-artifact workflow are pending. Language quality, WER/CER and runtime RSS not measured |

## Known gaps

The standard edition omits both optional private ASR packs. Native inference is optional and disabled by default. A pinned local model pack or explicit local model path with expected SHA-256 is required; a user hash is integrity, not authenticity. Multilingual language auto-detection is implemented, but language-level accuracy, WER/CER, runtime RAM use and performance are not measured. Cancellation cannot interrupt inference itself. Translation, diarization and TTS remain open. Audio-device enumeration and playback/recording are not verified on physical hardware; recording buffers, finalization and no-overwrite behavior have hardware-independent tests. The VAD is an adaptive RMS energy heuristic, not a neural model, and has not been checked against labeled speech/noise recordings. Transcript language and rate values are estimates. SRT supports strict basic timecodes and sequential indices; overlapping segments are allowed. Version 2 audio-workspace JSON projects preserve transcript cues and an optional relative linked-audio path, while version 1 projects remain readable; loading linked audio is an explicit user action. Projects do not preserve audio/transcript synchronization offsets. LRC import infers cue ends from the next timestamp and assigns three seconds to the final cue; export drops cue-end precision and flattens multiline text.

Hosted matrix, device, and packaging verification remain open. See [verification evidence](VERIFICATION.md) and [dependency review](DEPENDENCIES.md).
