# Optional offline Whisper model packs

Speech Studio's optional `asr` build supports two app-relative model packs: the existing English-only Tiny model and a compact multilingual Tiny model. Model binaries are not committed, fetched by this repository, or downloaded by the application. The packager only reads a local file that you obtained separately, checks its exact byte length and SHA-256, then writes a new ZIP without replacing an existing output.

## Assemble a pack offline

For English, retain the existing route:

```sh
python3 packaging/whisper_model_pack.py /path/to/ggml-tiny.en.bin whisper-tiny-en.zip
```

For multilingual Tiny, select its separate pinned manifest:

```sh
python3 packaging/whisper_model_pack.py /path/to/ggml-tiny.bin whisper-tiny-multilingual.zip --model-id whisper-tiny-multilingual
```

The multilingual artifact is pinned to Hugging Face repository revision `5359861c739e955e79d9a303bcbc70fb988958b1`. Its 77,691,713 bytes were downloaded and hashed locally; SHA-256 `be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21` matches that revision's Hugging Face LFS metadata. The manifest also records the model-source revision, OpenAI MIT license source and license SHA-256. No model binary is stored in Git.

Extract the selected pack next to the Speech Studio executable in an installation directory not writable by untrusted users. The files are placed under `engines/<model-id>/`; the application verifies the embedded byte count and SHA-256 before loading. In the ASR-enabled native audio workspace, select English Tiny or Multilingual Tiny; the selected pack is checked in a background task and the Transcribe button becomes available only after the manifest size and SHA-256 pass. The CLI `transcribe` operation keeps its blank-parameter English and `multilingual` selectors. For another model, enter its absolute file path and trusted SHA-256 separated by `|` in that CLI operation.

The multilingual model uses Whisper language auto-detection; English Tiny retains its fixed English mode. Whisper's upstream model card identifies Tiny as 39 million parameters and reports about 1 GiB required VRAM in its GPU benchmark. That is not a CPU RAM measurement. Speech Studio disables GPU inference and configures two CPU threads; its runtime RSS and speed have not been measured, so no minimum CPU RAM is claimed. The multilingual model file requires 77,691,713 bytes (about 74.1 MiB) of installed storage, plus the small manifest and license. During assembly, keep both the source file and new ZIP available; the tool uses temporary storage for the archive.

The model is offered under MIT as declared by OpenAI Whisper and the pinned conversion/model repository. Its exact license text is bundled with the pack. The multilingual model supports language identification upstream, but Speech Studio has not independently validated its language coverage or recognition quality. WER and CER have not been measured; no independently licensed multilingual reference corpus was used for this increment. Review the experimental transcript before export. Cancellation remains cooperative before and after native inference, not during it.

When preparing a portable preview, build with the `asr` feature and pass both the model file and, for multilingual, `--whisper-model-id whisper-tiny-multilingual` to `packaging/portable.py`. The default application package remains model-free. The preview packager checks that the binary advertises transcription and verifies the extracted model against its pinned manifest. Hardware, per-language quality, WER/CER, clean-device installation and physical audio behavior remain unverified.
