# Format and operation compatibility

The machine-readable registry is a development inventory, not an advertised capability list. Playback device behavior remains unverified on physical hardware.

| Operation | Route | Evidence status |
|---|---|---|
| scale-timing | Scale cue times from zero | Linux fixture-tested |
| trim-cues | Crop cues to a time interval | Linux fixture-tested |
| split-cue | Split a cue at an exact millisecond | Linux fixture-tested |
| merge-cues | Join consecutive cue text and timing | Linux fixture-tested |
| delete-cue | Remove and renumber a selected cue | Linux fixture-tested |
| shift-timing | Shift all subtitle cues by signed milliseconds | Linux fixture and CLI-tested |
| open-lrc | Import LRC metadata, offset and single/multiple timestamp lines into SRT draft | Linux offset, multitag, inferred-duration and invalid-input fixtures tested |
| export-lrc | Export cue start times with centisecond precision; joins multiline cue text | Linux fixture-tested; format is duration-lossy |
| waveform-svg | Export a peak SVG from mono 16 kHz PCM16 WAV | Linux fixture and CLI-tested |
| audio-info | 16 kHz mono signed PCM16 WAV | implemented-unverified |
| play-wav | Native Iced audio workspace with canvas waveform/playhead, WAV transport/seeking, synchronized SRT/JSON cue list, cue seeking, validated timing edits, undo/redo and version 1 project save; WAV limit 10 minutes/128 MiB | Linux decoder/bounds fixture passed; new headless cue/history/project tests added but not run locally; physical audio output not verified |
| detect-voice-activity | WAV energy intervals in JSON; adaptive RMS and hysteresis only | Linux silence and synthetic interval fixtures passed; speech/noise corpus not tested |
| analyze-transcript | SRT/draft to JSON language and text statistics, timing-derived estimated rate | Linux English/Russian, short text, Unicode words and overlap fixtures passed; values remain estimates |
| open-srt | UTF-8 SRT subset; editable draft | implemented-unverified |
| export-srt | Validated SRT output | implemented-unverified |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | implemented-unverified |
| transcribe | Optional CPU whisper-rs adapter, no bundled model | blocked |

See section 04-07 and 10 of the master specification for complete targets.
