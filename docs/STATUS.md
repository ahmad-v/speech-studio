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
| play-wav | Native Iced audio workspace: bounded local WAV/FLAC/MP3/OGG/AAC/AIFF decoding and playback, device enumeration/selection, bounded PCM16 WAV recording, live elapsed status and stop/finalize, waveform seek, synchronized transcript, signed transcript/audio offset, multiline cue text editing, literal search/replace, timing edits and shared undo/redo; v3 workspace JSON stores relative linked audio and sync offset; optional TTS controls generate and play a new local English WAV | Transcript edit, replacement, history and filter tests passed in Check, Integration and Native validation at code head `d74b7a8`; offset mapping, shared history and v3 compatibility tests added and pending hosted check; TTS controls are implemented in PR #32 but Rust compilation and real-model inference are pending; physical audio devices remain unverified |
| detect-voice-activity | Adaptive RMS and hysteresis intervals from 16 kHz mono PCM16 WAV, JSON report | Linux silence, synthetic activity and operation-output fixtures tested; energy heuristic only |
| analyze-transcript | Local language estimate, Unicode linguistic counts and subtitle-time rate estimate, JSON report | Linux English, Russian, short-text, overlap and operation-output fixtures tested; estimates only |
| replace-text | Exact transcript find and replace within cues | Linux fixture and CLI-tested |
| open-srt | UTF-8 SRT subset; editable draft | Linux fixture-tested |
| export-srt | Validated SRT output | Linux fixture-tested |
| export-vtt | Plain-text WebVTT from SRT input or draft | Linux fixture-tested |
| export-lrc | Export cue start times and text to LRC | Linux fixture-tested |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | Linux fixture-tested |
| translate-to-english | Experimental Whisper translation of speech audio into English using the private multilingual Tiny model; separate from general text translation | Model selector is restricted to multilingual and the native workspace has a dedicated action; inference quality and language coverage remain unverified |
| synthesize-speech | Experimental offline English VITS synthesis to a new PCM16 WAV with speed control, text/output bounds, progress, cooperative cancellation and playback from the audio workspace | Implemented in PR #32 with sherpa-onnx 2.0.3 and piper-plus-g2p; request and packer tests pass, while hosted Rust compilation and actual synthesis with the real model remain pending |
| verify-tts-pack | Verify app-relative voice/model/config/token/model-card files against pinned SHA-256 values | Implemented in PR #32; Rust execution test pending hosted verification |
| transcribe | Optional CPU whisper-rs adapter, private English or multilingual Tiny pack, or an explicit model path; multilingual pack pins its source bytes and uses language auto-detection. The native workspace verifies selected app-relative pack size and SHA-256 off the UI thread before enabling inference | Model-pack assembly passed in workflow 46127520782. Model-readiness UI and streaming hash checks were merged in PR #27; native optional-ASR builds/tests passed on five targets in run 46148917982. Verified installer/distribution changes merged in PR #28. The first portable preview produced Linux x64 and macOS Apple Silicon artifacts; Windows exposed CRLF checkout conversion of pinned JSON. Canonical LF metadata handling now has 12 passing Python tests and both actual model packs were independently extracted and SHA-256 checked with simulated Windows CRLF metadata; Windows x64 and ARM64 English and multilingual portable ASR bundle runs passed (46154698824, 46154871407). Multilingual inference accuracy, WER/CER, and runtime CPU memory/performance remain unmeasured |

## Known gaps

The standard edition omits both optional private ASR packs. Native inference is optional and disabled by default. A pinned local model pack or explicit local model path with expected SHA-256 is required; a user hash is integrity, not authenticity. Multilingual language auto-detection is implemented, but language-level accuracy, WER/CER, runtime RAM use and performance are not measured. Cancellation cannot interrupt inference itself. General text translation and diarization remain open. TTS synthesis source/UI is implemented in PR #32, but remains experimental and unverified until hosted Rust checks and actual inference pass. The selected upstream voice MODEL_CARD identifies its dataset as public domain but does not specify a model-weight license; the model pack is not bundled or distributed pending rights confirmation. Whisper speech-to-English translation has been implemented as an experimental unverified route that requires the multilingual Tiny pack; no translation corpus or language-quality validation has been run. Audio-device enumeration and playback/recording are not verified on physical hardware; recording buffers, finalization and no-overwrite behavior have hardware-independent tests. The VAD is an adaptive RMS energy heuristic, not a neural model, and has not been checked against labeled speech/noise recordings. Transcript language and rate values are estimates. SRT supports strict basic timecodes and sequential indices; overlapping segments are allowed. Version 3 audio-workspace JSON projects preserve transcript cues, an optional relative linked-audio path and the signed synchronization offset; version 1/2 projects remain readable with a zero offset, and loading linked audio is an explicit user action. Speaker labels are not stored. Cue text find/replace is case-sensitive and literal. LRC import infers cue ends from the next timestamp and assigns three seconds to the final cue; export drops cue-end precision and flattens multiline text.

The Windows ASR metadata correction is merged and targeted distribution builds passed; clean-host installed runtime inference and full package lifecycle checks remain open. Physical device tests and remaining package formats remain open. See [verification evidence](VERIFICATION.md) and [dependency review](DEPENDENCIES.md).
