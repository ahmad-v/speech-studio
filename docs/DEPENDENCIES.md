# Dependencies

See Cargo.toml  for exact direct version requirements. They are proposed, not resolved. No Cargo.lock exists because Cargo and registry access were unavailable. No transitive version, MSRV, license, security or build compatibility is asserted. The GitHub Actions configuration uses pinned checkout/upload SHAs as candidates; maintainer must review action source before enabling. No paid runners. Review RustSec advisories, native/model licensing, SBOM and build provenance before distribution.

Sources checked selectively: GitHub hosted runner table (https://docs.github.com/en/actions/reference/runners/github-hosted-runners), Iced 0.13.1 (https://docs.rs/crate/iced/0.13.1), lopdf 0.34.0 (https://docs.rs/crate/lopdf/0.34.0), calamine 0.26.1 (https://docs.rs/crate/calamine/0.26.1), whisper-rs 0.14.0 (https://docs.rs/crate/whisper-rs/0.14.0). Exact API signatures and all other versions still require cargo check. No model or binary is bundled.
