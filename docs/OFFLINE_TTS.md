# Offline English TTS (experimental)

The `tts` Cargo feature adds an offline English VITS path using the Rust
`sherpa-onnx` 2.0.3 API and `piper-plus-g2p` English phonemizer. It accepts at
most 8 KiB of text, writes a mono PCM16 WAV, caps output at 180 seconds, checks
the model files against SHA-256 values, and refuses to replace an existing
file. The native audio workspace can generate and play a new WAV. This voice is
single-speaker English; it is not voice cloning or multilingual synthesis.
Inference is in-process native code, not an OS sandbox. The sherpa build script
retrieves versioned native archives for the selected build target; pinning and
verifying those archive digests in the distribution workflow remains required
before shipping TTS-enabled binaries.

## Voice asset provenance and distribution status

The selected upstream pack is
[`vits-piper-en_US-ljspeech-medium-int8`](https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-ljspeech-medium-int8.tar.bz2).
Its 21,090,429-byte source archive is pinned in
[`manifest.json`](../packaging/tts-model/manifest.json) by SHA-256. The retained
upstream `MODEL_CARD` identifies the LJ Speech dataset as public domain. That
statement describes the training data; it does not expressly state a license
for the separately distributed model weights. Weight redistribution rights
are therefore unresolved. Do not bundle, rehost, or publish this voice pack
until that license has been independently confirmed. The model loader and
local packer are implemented, but the voice distribution is not a completed
release capability.

To assemble a local development pack after obtaining the pinned source archive,
run:

```sh
python3 packaging/tts_model_pack.py /path/to/vits-piper-en_US-ljspeech-medium-int8.tar.bz2 /tmp/tts-pack
```

The script verifies the archive digest and extracts only four allowlisted
regular files (ONNX model, model config, tokens and model card), checking each
file's exact size and digest. It deliberately excludes the unused eSpeak data.
Place the resulting `engines/tts-en-us-ljspeech-medium-int8` directory beside
the application executable. Runtime inference does not access the network.

The voice pack is about 19.4 MiB before ZIP compression. Inference is CPU-only
with two configured threads. Runtime, peak memory and pronunciation quality
have not yet been benchmarked or independently validated. Rust compilation and
actual inference still require hosted verification with the `tts` feature and
the private model pack; no local Cargo build is claimed for this increment.
