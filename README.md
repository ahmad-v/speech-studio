# Speech Studio

Offline speech and transcript workbench. This is a development snapshot (0.1.0-alpha.1), not a verified release or installer.

This independent studio uses Localwright at source level. There is no separate mandatory Localwright installation.

## Current functionality

- Local PCM16 WAV inspection, preparation, peak waveform export, and an energy-based activity estimate.
- Bounded native playback and waveform navigation for WAV, FLAC, MP3, OGG, AAC/M4A and AIFF decoders. MP4 audio is decoded through a bounded in-memory Symphonia route so malformed or incomplete container length metadata cannot reach Rodio's panicking MP4 initialization path.
- A separate Iced audio workspace (`speech-studio --audio`) provides bounded offline multi-format playback, selectable local input devices, native PCM16 WAV recording with explicit stop and live elapsed status, a clickable waveform/playhead, synchronized transcript cue seeking and editable timings with undo/redo.
- UTF-8 SRT, WebVTT, LRC, JSON, and plain-text exchange, with bounded transcript and cue-range editing.
- Unicode-aware transcript summaries and local language estimates.
- Optional CPU Whisper integration. The model is not included in the baseline application or this repository; no runtime download occurs.

The project is intended to remain free, local and offline, without tracking, advertising, subscriptions or premium features. See [the manifesto](MANIFESTO.md) for the motivation behind it.

## Development and verification

The Linux x64 source suite has fixture coverage. Recording buffer, WAV finalization and non-overwrite behavior have hardware-independent tests. Device enumeration, physical playback and recording are not hardware-verified. Five-target checks for the current feature head, engine-enabled target coverage, multilingual accuracy, packaging, and installer lifecycle are still incomplete. The energy-based activity estimate is not neural VAD. See the [build notes](docs/BUILD.md), [CI workflow roles](docs/CI.md), [status](docs/STATUS.md), [architecture](docs/ARCHITECTURE.md), and [manual](docs/MANUAL.md). The [master specification](MASTER_SPECIFICATION.md) is authoritative for remaining work.

## Citation and contributions

If the software is useful in your research, please consider citing its version once a verified release and actual repository URL exist. See `CITATION.cff`. No DOI has been assigned. Code, testing, translations and documentation are welcome. No donation account has been configured.

Original code candidate license: MIT OR Apache-2.0, pending maintainer approval and dependency review. AI-assisted source requires human review before publication.
