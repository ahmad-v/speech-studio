# Offline manual | Speech Studio

Development source only. Launch the GUI after compiling. Choose an operation, enter local input path and a new output path where needed, set the parameter as described under the operation selector, then Run. Status and errors are shown below. Cancel requests cooperative cancellation. The draft editor accepts transcript or plain text. Choose a theme to change workbench colors and control borders. UI scale adjusts the controls from 80% to 300% for the current session. Paths are manually typed in this increment; native dialogs and full accessibility remain pending.

- `open-vtt`: Imports simple UTF-8 cues. Cue settings, regions, styles, and timed markup are outside this route. Cue identifiers and NOTE blocks are omitted.
- `prepare-wav`: Converts mono or stereo PCM16 WAV at 16, 32, or 48 kHz to a new 16 kHz mono WAV. Downsampling averages groups of input frames.
- `audio-info`: 16 kHz mono signed PCM16 WAV.
- `open-srt`: UTF-8 SRT subset; editable draft.
- `export-srt`: Validated SRT output.
- `export-vtt` writes WebVTT from SRT input or an edited draft. Cue text is exported as literal text.
- `transcribe`: For an ASR-enabled build, leave the parameter blank to use the [private English model pack](WHISPER_MODEL_PACK.md).
- `export-txt`: Plain transcript text.

Limitations: No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Private Tiny English pack or explicit local model path with expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

No output overwrite is allowed. No accounts, ads, telemetry or runtime downloads are intended. Do not use this unverified snapshot with valuable or hostile input. Citation and donations are voluntary.

WAV preparation downmixes and uses weighted frame averaging for supported PCM16 inputs. High-frequency content can alias during downsampling.
