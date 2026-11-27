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
| play-wav | Native Iced audio workspace: bounded local WAV/FLAC/MP3/OGG/AAC/AIFF decoding and playback, clickable waveform/playhead seek, SRT/JSON cue synchronization and seeking, validated cue timing edits, versioned project save, undo/redo | WAV decoder/bounds fixture passed before multi-format change; new codec matrix pending hosted Integration; ALSA development library is unavailable in this local environment; physical audio-device playback not verified |
| detect-voice-activity | Adaptive RMS and hysteresis intervals from 16 kHz mono PCM16 WAV, JSON report | Linux silence, synthetic activity and operation-output fixtures tested; energy heuristic only |
| analyze-transcript | Local language estimate, Unicode linguistic counts and subtitle-time rate estimate, JSON report | Linux English, Russian, short-text, overlap and operation-output fixtures tested; estimates only |
| replace-text | Exact transcript find and replace within cues | Linux fixture and CLI-tested |
| open-srt | UTF-8 SRT subset; editable draft | Linux fixture-tested |
| export-srt | Validated SRT output | Linux fixture-tested |
| export-vtt | Plain-text WebVTT from SRT input or draft | Linux fixture-tested |
| export-lrc | Export cue start times and text to LRC | Linux fixture-tested |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | Linux fixture-tested |
| transcribe | Optional CPU whisper-rs adapter, optional private Tiny English pack or an explicit model path | Linux x64 app-relative model and export smoke-tested |

## Known gaps

The standard edition omits the optional private ASR pack. Native inference is optional and disabled by default. Private Tiny English pack or explicit local model path with expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. Broader media decoding, recording, language accuracy across held-out fixtures, WER, translation, diarization and TTS remain open. Native audio-device playback has not been verified on physical hardware. The new VAD is an adaptive RMS energy heuristic, not a neural model, and has not been checked against labeled speech/noise recordings. Transcript language and rate values are estimates. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed. Audio-workspace JSON projects store transcript cues only and do not preserve source audio paths or audio/transcript synchronization offsets. LRC import infers cue ends from the next timestamp and assigns three seconds to the final cue; export drops cue-end precision and flattens multiline text.

Hosted matrix, device, and packaging verification remain open. See [verification evidence](VERIFICATION.md) and [dependency review](DEPENDENCIES.md).
