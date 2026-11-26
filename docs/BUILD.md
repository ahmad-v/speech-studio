# Building the alpha snapshot

Extract all five source archives into one directory as siblings. Use Rust toolchain from rust-toolchain.toml. Run `cargo generate-lockfile`, review dependency resolution, then `cargo fmt --all`, `cargo check --locked --all-targets`, `cargo test --locked`, `cargo clippy --locked --all-targets`. Run `cargo run --locked` for the native GUI, or `cargo run --locked --no-default-features -- --cli --help` for the headless handler.

On Linux, development needs platform libraries for Iced (X11/Wayland, keyboard and graphics). CI installs candidate build dependencies. No system packages are installed by applications. Speech optional inference requires native C/C++ build support and libclang where needed; use `cargo check --locked --features asr` after model/backend review.

## Integration

Before CI, replace sibling path dependencies with exact git dependencies using actual approved GitHub URL and immutable revision, or keep the sibling checkout workflow with the same pinned revision. Set `integration/localwright.json` to the real repository and commit. The supplied CI checks that SHA. Do not invent identifiers.

The check workflow intentionally fails if Cargo.lock is absent. The manually dispatched bootstrap workflow produces a candidate lockfile artifact for review, never commits it. Commit reviewed lockfiles. The check gate includes formatting, so run rustfmt before first green check. No package workflow can publish a release. No default OS support versions have been validated.
