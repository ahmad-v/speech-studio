# Offline text-to-speech (experimental)

The optional `tts` feature adds native offline English synthesis through the Rust
`sherpa-onnx` 2.0.3 API and Kitten TTS Nano 0.8 int8. The Iced audio workspace
offers eight named voices, speed control, progress, cancellation, playback, and
new PCM16 WAV output. Input is bounded to 8 KiB and generated audio to 180
seconds. Existing outputs are never replaced. The application discovers its
model pack beside the executable and checks every asset against SHA-256 values
before inference.

## Model and voice provenance

The model is `KittenML/kitten-tts-nano-0.8-int8` at immutable Hugging Face
revision `84781d74e29ee25217551556398b42f80593a813`. That repository's pinned
model card marks the model Apache-2.0 and lists the same eight voices shipped
in its `voices.npz`. The sherpa-compatible int8 ONNX and voice files are
obtained from `kitten-nano-en-v0_8-int8.tar.bz2` (31,220,690 bytes,
SHA-256 `6fa5be852612ce761094ba74ee6123b4fc4acfefa79bf64dc63acae4a83af2fd`).
The manifest records the model/voice file sizes and hashes. Kitten Nano is
English-only, about 15 million parameters, with a 24 kHz output rate.

The pronunciation lexicon is generated deterministically from CMUdict at
commit `74790861f652b15e4ac49015a90074ad62a27690` (BSD-style Carnegie Mellon
license). The generated lexicon is pinned at 3,972,115 bytes and SHA-256
`c7b7d6bd6db10cf90af8779c95e81e7d85d1c19a711c90b9bb3357b084b38b74`; its
required CMU copyright, conditions, and disclaimer are packaged as
`CMUDICT-LICENSE`. The packer rejects phones not present in Kitten's token
inventory. The generated lexicon uses a documented ARPABET-to-IPA mapping; its
pronunciation quality and out-of-vocabulary behavior have not yet been
independently assessed.

Sherpa-ONNX 2.x removed its eSpeak-NG and piper-phonemize dependency. This pack
does not include the release archive's unused `espeak-ng-data`. The synthesis
path uses the sherpa Kitten lexicon interface and makes no GPL phonemizer claim.
The sherpa runtime's native archive and transitive binary linkage still need
target-by-target distribution review before a TTS-enabled release.

## Build an offline development pack

Obtain the model archive and `cmudict.dict` plus `LICENSE` from the exact commit
above. The packer verifies the model archive size/hash, dictionary hash,
license hash, generated lexicon, allowlisted archive members, and all output
asset hashes. It never extracts the unused archive subtrees:

```sh
python3 packaging/tts_model_pack.py \
  /path/to/kitten-nano-en-v0_8-int8.tar.bz2 \
  /path/to/cmudict.dict \
  /path/to/cmudict-LICENSE \
  /tmp/tts-pack
```

Place the resulting `engines/tts-en-us-kitten-nano-v0_8-int8` directory beside
the executable. Inference does not access the network. The compressed source
archive is about 29.8 MiB; model, voices, tokens, lexicon, and licensing/model
card files occupy about 30.8 MiB before packaging. No application download or
runtime package manager is required. The manual `Offline TTS development
bundle` workflow builds an unsigned Linux x64 bundle, extracts it, verifies the
installed pack through the application, synthesizes a sample, and checks the
resulting WAV before uploading a short-lived artifact. That package workflow
has not yet been run; other target-specific bundles remain future distribution
work.

## Verification status

Source checks verify manifest fields, archive boundaries, source provenance,
CMUdict-to-IPA conversion, voice IDs, file hashes, symlink rejection, and
no-overwrite output behavior. Six packer tests and all 18 Python packaging tests
pass locally. The pinned model, CMUdict, and license were assembled into a
private pack successfully; this validates the packer against the real inputs,
not speech generation. Hosted Rust feature compilation and actual offline
inference smoke verification remain pending. No accuracy, pronunciation,
runtime, memory, or physical audio-device verification is claimed here.
