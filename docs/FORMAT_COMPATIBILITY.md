# Format and operation compatibility

Nothing is verified on a native target. The machine-readable registry is a development inventory, not an advertised capability list.

| Operation | Route | Evidence status |
|---|---|---|
| audio-info | 16 kHz mono signed PCM16 WAV | implemented-unverified |
| open-srt | UTF-8 SRT subset; editable draft | implemented-unverified |
| export-srt | Validated SRT output | implemented-unverified |
| open-json / export-json | Versioned UTF-8 transcript exchange with millisecond timings | Linux fixture and CLI-tested |
| export-txt | Plain transcript text | implemented-unverified |
| transcribe | Optional CPU whisper-rs adapter, no bundled model | blocked |

See section 04-07 and 10 of the master specification for complete targets.
