# Offline text translation

Speech Studio includes a Rust-native Marian inference route for one explicitly
bounded language direction: English to French. It is distinct from Whisper's
speech-to-English mode. The translation UI is available from the audio
workspace or by launching `speech-studio --translate-text`; the CLI operation
reads a UTF-8 `.txt` source and writes a new `.txt` file. Both routes run on
the local CPU and never request a network connection.

## Model and license

The pack uses `Helsinki-NLP/opus-mt-en-fr`, pinned to Hugging Face revision
`dd7f6540a7a48a7f4db59e5c0b9c42c8eea67f18`. Its pinned model card declares
Apache-2.0. The packer includes that model card and the full Apache license
text. The model weights are approximately 301 MB; two SentencePiece vocabulary
files, the card, manifest and notices are included as well. No model binary is
committed to this repository. `packaging/translation-model/manifest.json`
records exact sizes and SHA-256 values. The engine and tokenizer components
are separately inventoried in `DEPENDENCIES.md` and `THIRD_PARTY_NOTICES.md`.

The official model card reports historical benchmark scores for its model.
Those published figures are not a Speech Studio validation result. The
application currently exposes greedy decoding rather than the upstream model's
four-beam generation setting. Translation quality, runtime, memory usage and
domain-specific accuracy have not yet been measured independently. Review
names, numbers, idioms and technical text before relying on the output.

## Build and install the expansion pack

The model is an optional, free offline asset. Package jobs download only the
exact pinned source assets over HTTPS and reject mismatched hashes:

```sh
python3 packaging/translation_model_pack.py assemble /path/to/opus-mt-en-fr.zip
python3 packaging/translation_model_pack.py verify /path/to/opus-mt-en-fr.zip
python3 packaging/translation_model_pack.py install \
  /path/to/opus-mt-en-fr.zip /path/to/SpeechStudio
```

The generated archive contains `engines/opus-mt-en-fr-v1`. A user can extract
it with the operating system archive utility into the directory containing the
Speech Studio executable. The GUI **Verify offline model pack** action checks
the manifest, file types, exact lengths and SHA-256 values before inference.
There is no runtime downloader or fallback to a system installation; end users
do not need Python, PyTorch, Rust or a package manager. Keep the pack beside
the application when moving or installing Speech Studio.

The `install` subcommand is a development/packaging convenience; all Python
script commands in this section are for maintainers preparing or validating an
asset pack. To assemble from already-acquired assets without network access,
pass `--cache-dir` containing the files named by the manifest. Every input
must still match its pinned size and SHA-256.

## Bounds and status

- Input is limited to 8 KiB and 384 source subword tokens.
- Output generation is limited to 384 tokens; inference checks cancellation
  between generated tokens.
- Output exports use create-new semantics and preserve any existing file.
- The pack and inference source are integrated, but a hosted clean-pack
  inference smoke and multilingual-quality benchmark remain required before
  the capability can be accepted as fully verified.
- The current model covers English-to-French only. It is not a general
  multilingual translation engine, and it is not a speech translation model.
