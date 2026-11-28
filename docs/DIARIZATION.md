# Offline speaker diarization (development pack)

Speech Studio now has a Rust diarization module for sherpa-onnx 2.0.3. It
combines pyannote segmentation, 3D-Speaker ERes2Net embeddings,
and sherpa-onnx clustering. The operation accepts only bounded 16 kHz mono
PCM16 WAV input (up to ten minutes), runs locally, checks cancellation before
and after the native inference call, and validates all installed model hashes
before loading native inference. Native inference itself is currently not
interruptible.

The model files are not checked into Git. To assemble an unsigned local
development bundle, download the pinned upstream assets and run:

```sh
curl -fL --retry 3 \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2 \
  -o /tmp/speaker-segmentation.tar.bz2
curl -fL --retry 3 \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx \
  -o /tmp/speaker-embedding.onnx
python3 packaging/diarization_pack.py \
  /tmp/speaker-segmentation.tar.bz2 /tmp/speaker-embedding.onnx /tmp/speaker-pack
```

The packager checks the exact source archive/model hashes, extracts only the
required model and notices, verifies each emitted asset, and produces
`speaker-diarization-pyannote3-3dspeaker-v1.zip`. Install the extracted
`engines/speaker-diarization-pyannote3-3dspeaker-v1` directory alongside the
application executable. Inference does not access the network. This does not
yet mean that the standard studio installers include the pack.

## Provenance and model licenses

The segmentation model is the int8 ONNX conversion of
[`pyannote/segmentation-3.0`](https://huggingface.co/pyannote/segmentation-3.0)
from the sherpa-onnx speaker-segmentation release. The upstream model declares
MIT. The embedding model is the 3D-Speaker ERes2Net-Base model from Alibaba
DAMO Academy, distributed by the sherpa-onnx speaker-recognition release; its
upstream [3D-Speaker project](https://github.com/modelscope/3D-Speaker)
declares Apache-2.0. Each license notice and the model card are included in the
pack. Engine licensing and notices are handled as
part of the sherpa-onnx dependency/package notices. The Revai/Reverb model is
excluded because its model agreement limits it to non-production use.

`packaging/diarization/manifest.json` records model identities, source URLs,
SHA-256, byte lengths and separate licenses. The hashes were checked against
the downloaded upstream artifacts during implementation. The embedding model
card identifies 3D-Speaker training data; no training dataset files are in the
pack, and dataset terms are separate from the model's stated license.

## Output meaning and verification status

Inference outputs time spans with recording-local speaker indices and backend
confidence scores. These indices only distinguish clusters within a recording;
they are not identity recognition. Output can be mapped to transcript cues by
maximum interval overlap and serialized as a versioned transcript with editable
labels. Uncovered cues remain unlabeled. Overlapping turns are simplified to
one label per transcript cue. Short turns, overlapping voices, noise, similar
voices and language coverage can affect quality. The confidence score is not a
calibrated probability.

Packaging tests cover deterministic asset allowlisting, model hash rejection,
license inclusion and archive-link rejection. The actual upstream model files
were downloaded and assembled on 2026-11-28; the 41,152,599-byte ZIP passed
the packager and contained both model files and both license notices. The
module is not yet registered in the operation/workspace UI. Native Rust
compilation, end-to-end speaker inference, UI integration and reference-corpus
quality measurements are pending. There is no diarization error-rate claim.
