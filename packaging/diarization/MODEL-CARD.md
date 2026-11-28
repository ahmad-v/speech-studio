# Offline speaker diarization model pack

Pack ID: `speaker-diarization-pyannote3-3dspeaker-v1`
Inference engine: sherpa-onnx 2.0.3 (separately licensed; see dependency notices).
Input: mono 16 kHz audio. The application limits a job to ten minutes.

## Model assets and rights

* `segmentation.int8.onnx` is the int8 ONNX conversion of
  [`pyannote/segmentation-3.0`](https://huggingface.co/pyannote/segmentation-3.0),
  distributed by the sherpa-onnx `speaker-segmentation-models` release. The
  upstream model declares MIT; its notice is included as
  `LICENSE-pyannote-MIT`.
* `speaker-embedding.onnx` is the unmodified sherpa-onnx release of
  Alibaba DAMO Academy's 3D-Speaker ERes2Net-Base model
  [`damo/speech_eres2net_base_sv_zh-cn_3dspeaker_16k`](https://modelscope.cn/models/damo/speech_eres2net_base_sv_zh-cn_3dspeaker_16k/summary).
  The [3D-Speaker project](https://github.com/modelscope/3D-Speaker) declares
  Apache-2.0. Its license is included as
  `LICENSE-3dspeaker-Apache-2.0`.

The segmentation model card reports training on the combination of AISHELL,
AliMeeting, AMI, AVA-AVD, DIHARD, Ego4D, MSDWild, REPERE and VoxConverse.
Training datasets are not included. Dataset terms do not replace the model
licenses above. sherpa-onnx provides the model conversions and C/Rust runtime;
its own license and third-party notices are distributed separately.

Both model files and the upstream source archive are pinned by SHA-256 in
`manifest.json`. The application verifies the installed files before loading
them. The upstream model artifacts are obtained from the official sherpa-onnx
release assets. This pack does not fetch anything during inference.

## Intended use and limits

This is offline speaker diarization: it estimates temporal speaker clusters
within one recording. Cluster numbers are local to that recording and do not
identify a person's real-world identity. The model was trained on varied
meeting and speech corpora but no independent project benchmark is yet claimed.
Overlapping speech, short turns, noise, reverberation and similar voices may
cause missed or incorrect assignments. Review and correct speaker labels.
Confidence values are backend scores, not calibrated probabilities.

The embedding model is described as Chinese/English (zh-cn) speaker
verification. Do not infer broad language or demographic coverage from this
model card. A reproducible multilingual diarization evaluation remains
outstanding.
