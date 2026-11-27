# Dependencies

Cargo manifests list the exact direct requirements. The committed lockfile records the resolved versions and registry checksums. Builds use Rust 1.88.0 from `rust-toolchain.toml`. The CI checkout action is pinned to its upstream v4.2.2 commit.

A preliminary RustSec review used advisory database revision `c9f6c91a44dbad9d71742329e9df061279f9c4be`. The pinned Localwright revision uses Iced’s Tiny Skia software renderer and removes the affected `lru 0.12.5` dependency path (RUSTSEC-2026-0002 and RUSTSEC-2026-0253). Full license review, SBOM and native dependency review remain open.

The optional ASR feature pins Whisper 0.14.4 because 0.14.0 was yanked. The repository ships without a model or model binary.

Transcript analysis adds direct, exact pins to `whatlang 0.18.0` (MIT) for offline language estimates and `unicode-segmentation 1.13.3` (MIT OR Apache-2.0) for Unicode-aware text counts. The Cargo lockfile captures the language detector's transitive dependencies. Generated portable notices must be rebuilt and reviewed for each package configuration; this records declared crate licenses, not a full native-linkage or asset review.
