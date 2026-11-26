# Building the alpha snapshot

Run these commands from the repository root. Rustup selects the toolchain in `rust-toolchain.toml`. The first Cargo build fetches dependencies, including the exact Localwright Git revision recorded in `Cargo.toml` and `integration/localwright.json`.

```sh
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

On Linux, install the X11 and Wayland development libraries listed in the check workflow. Run `cargo run --locked` from a graphical session to launch the app. Use `cargo run --locked --no-default-features -- --cli --help` for the headless interface.

Update both Localwright revision records and regenerate `Cargo.lock` when advancing the shared dependency.

The optional `asr` feature requires native C/C++ build tools, CMake and libclang. Run `cargo check --locked --no-default-features --features asr` to check compilation. Inference requires an explicitly selected local model and its SHA-256.
