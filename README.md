# Speech Studio

Offline speech and transcript workbench. This is a development snapshot (0.1.0-alpha.1), not a verified release or installer.

This independent studio uses Localwright at source level. There is no separate mandatory Localwright installation.

## Current functionality

- Local PCM16 WAV inspection, preparation, peak waveform export, and an energy-based activity estimate.
- Bounded native WAV playback for mono or stereo PCM16 files; host audio device behavior is not hardware-verified.
- A separate Iced audio workspace (`speech-studio --audio`) provides bounded WAV playback, a canvas waveform/playhead, SRT and JSON transcript cue synchronization, cue seeking and validated timing edits with undo/redo.
- UTF-8 SRT, WebVTT, LRC, JSON, and plain-text exchange, with bounded transcript and cue-range editing.
- Unicode-aware transcript summaries and local language estimates.
- Optional CPU Whisper integration. The model is not included in the baseline application or this repository; no runtime download occurs.

The project is intended to remain free, local and offline, without tracking, advertising, subscriptions or premium features. See [the manifesto](MANIFESTO.md) for the motivation behind it.

## Development and verification

The Linux x64 source suite has fixture coverage. The WAV decoder/bounds fixture passed in GitHub Integration; the native audio device and platform playback behavior are not hardware-verified. Headless model and transport logic tests for this increment have not run locally because Cargo is unavailable in this workspace session and disk is constrained. Five-target checks, engine-enabled target coverage, recording, multilingual accuracy, packaging, and installer lifecycle are still incomplete. The energy-based activity estimate is not neural VAD. See the [build notes](docs/BUILD.md), [CI workflow roles](docs/CI.md), [status](docs/STATUS.md), [architecture](docs/ARCHITECTURE.md), and [manual](docs/MANUAL.md). The [master specification](MASTER_SPECIFICATION.md) is authoritative for remaining work.

## Citation and contributions

If the software is useful in your research, please consider citing its version once a verified release and actual repository URL exist. See `CITATION.cff`. No DOI has been assigned. Code, testing, translations and documentation are welcome. No donation account has been configured.

Original code candidate license: MIT OR Apache-2.0, pending maintainer approval and dependency review. AI-assisted source requires human review before publication.
