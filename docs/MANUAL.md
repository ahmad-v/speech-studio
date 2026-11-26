# Offline manual | Speech Studio

Development source only. Launch the GUI after compiling. Choose an operation, enter local input path and a new output path where needed, set the parameter as described under the operation selector, then Run. Status and errors are shown below. Cancel requests cooperative cancellation. The draft editor accepts transcript or plain text. Choose a theme to change workbench colors and control borders. UI scale adjusts the controls from 80% to 300% for the current session. Paths are manually typed in this increment; native dialogs and full accessibility remain pending.

- `audio-info`: 16 kHz mono signed PCM16 WAV.
- `open-srt`: UTF-8 SRT subset; editable draft.
- `export-srt`: Validated SRT output.
- `export-vtt` writes WebVTT from SRT input or an edited draft. Cue text is exported as literal text.
- `export-txt`: Plain transcript text.
- `transcribe`: Optional CPU whisper-rs adapter, no bundled model.

Limitations: No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Explicit local model path and expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

No output overwrite is allowed. No accounts, ads, telemetry or runtime downloads are intended. Do not use this unverified snapshot with valuable or hostile input. Citation and donations are voluntary.
