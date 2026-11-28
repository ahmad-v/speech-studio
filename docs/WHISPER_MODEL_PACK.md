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

Install a downloaded pack with the verifier below, passing the folder that contains the Speech Studio executable. The script checks that the archive contains exactly the pinned manifest, license, and model; normalizes CRLF checkout endings to canonical LF for pinned text metadata; verifies model byte length and SHA-256; rejects unexpected paths; and refuses to overwrite an already installed model. It writes `engines/<model-id>/` beside the application. Run it with permissions to write the application folder, and keep that folder protected from untrusted writes. The application also re-verifies the model before inference. In the ASR-enabled native audio workspace, select English Tiny or Multilingual Tiny; a background check enables Transcribe only after the model verifies. The CLI `transcribe` operation retains its blank-parameter English and `multilingual` selectors. For another model, enter its absolute file path and trusted SHA-256 separated by `|` in that CLI operation.

```sh
python3 packaging/install_whisper_model_pack.py whisper-tiny-en.zip /path/to/SpeechStudio
# For the multilingual pack:
python3 packaging/install_whisper_model_pack.py whisper-tiny-multilingual.zip /path/to/SpeechStudio --model-id whisper-tiny-multilingual
```

The multilingual model uses Whisper language auto-detection; English Tiny retains its fixed English mode. Whisper's upstream model card identifies Tiny as 39 million parameters and reports about 1 GiB required VRAM in its GPU benchmark. That is not a CPU RAM measurement. Speech Studio disables GPU inference and configures two CPU threads; its runtime RSS and speed have not been measured, so no minimum CPU RAM is claimed. The multilingual model file requires 77,691,713 bytes (about 74.1 MiB) of installed storage, plus the small manifest and license. During assembly, keep both the source file and new ZIP available; the tool uses temporary storage for the archive.

The model is offered under MIT as declared by OpenAI Whisper and the pinned conversion/model repository. Its exact license text is bundled with the pack. The multilingual model supports language identification upstream, but Speech Studio has not independently validated its language coverage or recognition quality. WER and CER have not been measured; no independently licensed multilingual reference corpus was used for this increment. Review the experimental transcript before export. Cancellation remains cooperative before and after native inference, not during it.

The manual **Offline ASR portable distribution preview** workflow builds an optimized ASR-enabled portable bundle for all five native targets with one selected model included. It verifies the model archive and each target bundle and uploads unsigned development artifacts for 14 days. These are not signed release packages. The baseline **Distribution preview** remains model-free. Hardware, per-language quality, WER/CER, clean-device installation and physical audio behavior remain unverified.
