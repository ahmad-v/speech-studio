# Speech Studio

Local speech and transcript workbench. This is a development snapshot (0.1.0-alpha.1), not a verified release or installer.

This independent studio uses Localwright at source level. There is no separate mandatory Localwright installation.

## What is in this snapshot

- 16 kHz mono signed PCM16 WAV (implemented-unverified).
- UTF-8 SRT subset; editable draft (implemented-unverified).
- Validated SRT output (implemented-unverified).
- Plain transcript text (implemented-unverified).
- Optional CPU whisper-rs adapter, no bundled model (blocked).

The project is intended to remain free, local and offline, without tracking, advertising, subscriptions or premium features. See [the manifesto](MANIFESTO.md) for the motivation behind it.

## Development

See [build notes](docs/BUILD.md), [status](docs/STATUS.md), [architecture](docs/ARCHITECTURE.md) and [manual](docs/MANUAL.md). The complete [master specification](MASTER_SPECIFICATION.md) defines the remaining requirements. No compilation, platform support or package has been verified in this snapshot.

## Citation and contributions

If the software is useful in your research, please consider citing its version once a verified release and actual repository URL exist. See `CITATION.cff`. No DOI has been assigned. Code, testing, translations and documentation are welcome. No donation account has been configured.

Original code candidate license: MIT OR Apache-2.0, pending maintainer approval and dependency review. AI-assisted source requires human review before publication.
