# Verification evidence

Static checks performed on source artifacts only:

- speech-studio: specification exact; TOML/JSON/YAML/CFF syntax, Python AST, XML/SVG/WiX well-formedness passed (not schema validation).
- speech-studio: exact sibling dependency paths/version and 15 unique packaging rows passed.
- Speech fixture: Python wave independently verified 1 s mono 16 kHz PCM16 silence.
- speech-studio: packaging architecture helper accepted synthetic ARM64 PE header and rejected x64 mismatch (not an executable test).

No Rust compilation or runtime tests executed. Cargo/rustc not installed; DNS lookup of static.rust-lang.org and index.crates.io failed. CI, package builds, signing, SBOM, license and security review remain pending.
