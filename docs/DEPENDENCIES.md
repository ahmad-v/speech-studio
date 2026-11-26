# Dependencies

Cargo manifests list the exact direct requirements. The committed lockfile records the resolved versions and registry checksums. Builds use Rust 1.88.0 from `rust-toolchain.toml`. The CI checkout action is pinned to its upstream v4.2.2 commit.

A preliminary RustSec review used advisory database revision `c9f6c91a44dbad9d71742329e9df061279f9c4be`. The GUI graph still includes `lru 0.12.5` through `iced_glyphon` (RUSTSEC-2026-0002 and RUSTSEC-2026-0253). Track an upstream update and assess reachability before distribution. Full license review, SBOM and native dependency review remain open.

The optional ASR feature pins Whisper 0.14.4 because 0.14.0 was yanked. The repository ships without a model or model binary.
