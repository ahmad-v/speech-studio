# Format and operation compatibility

Nothing is verified on a native target. The machine-readable registry is a development inventory, not an advertised capability list.

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
| open-srt | UTF-8 SRT subset; editable draft | implemented-unverified |
| export-srt | Validated SRT output | implemented-unverified |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | implemented-unverified |
| transcribe | Optional CPU whisper-rs adapter, no bundled model | blocked |

See section 04-07 and 10 of the master specification for complete targets.
