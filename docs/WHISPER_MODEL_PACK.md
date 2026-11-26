# Private Whisper Tiny English model pack

Speech Studio's optional `asr` feature supports an application-relative English model pack. Use the pinned GGML model and license listed in [the source manifest](../packaging/whisper-model/manifest.json). The pack assembler accepts a previously obtained local file and checks its exact length and SHA-256 before writing a new ZIP.

```sh
python3 packaging/whisper_model_pack.py /path/to/ggml-tiny.en.bin whisper-tiny-en.zip
```

Extract the pack beside the Speech Studio executable in an installation directory that other users cannot modify. The expected path is `engines/whisper-tiny-en/ggml-tiny.en.bin`. In an ASR-enabled build, select Transcribe and leave its parameter blank to use this English model. The application verifies the bytes against its built-in pin before loading. To use a separately supplied model, enter its absolute path and a trusted SHA-256 separated by `|`.

When preparing a combined portable bundle, build with the default features plus `asr`, then pass `--whisper-model /path/to/ggml-tiny.en.bin` to `packaging/portable.py`. The packager requires an executable that advertises the transcription route and verifies the model after extraction. The dependency inventory includes the ASR feature graph and the vendored whisper.cpp license.

The pack contains model data, provenance and the model's MIT license. The CPU inference engine is provided by the ASR-enabled application build. The baseline installer still omits ASR. The application does not fetch models during use.

Both explicit and app-relative model lookup have Linux x86_64 offline smoke evidence. The app-relative test extracted the generated pack alongside a locked ASR-enabled CLI build, transcribed a short reference WAV into a two-cue draft without passing a model path, and exported that draft via the SRT route. Tracing recorded no socket or connect calls while socket creation was blocked. A truncated model copy failed the SHA-256 check. This evidence applies to a staged local binary. Hardware, clean-device, ASR-enabled installer and broad model quality checks remain pending.

The Linux x64 ASR-enabled portable archive passed checksum verification and a second smoke test from the extracted payload. Its pinned application-relative model transcribed the reference WAV and exported the reviewed draft as two SRT cues. Socket creation was blocked and the trace showed no socket or connect calls.
