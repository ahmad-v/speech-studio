# sherpa-onnx-sys vendored build support

This directory contains the published `sherpa-onnx-sys` 2.0.3 crate source
(`Apache-2.0`, crates.io package checksum
`10f04396502e412187635ed9f0eb1b6ee9f9680d9bd04cdfdd4c8b709019c1f1`). The
`src/`, `build.rs`, README, manifest and license are unchanged from that crate.

The sole local manifest change narrows the build-only `zip` dependency to
exactly 9.0.0, disables its unrelated default codec/encryption features, and
enables `deflate`. The upstream build script uses ZIP extraction only for
`.xcframework.zip` archives; normal desktop target archives use BZip2/TAR.
`deflate` retains support for ordinary compressed ZIP entries. ZIP 9.0.0
declares Rust 1.88 support, while the latest ZIP 9.9.0 and its default AES 0.9
dependency require Rust 1.89. This targeted build-dependency configuration
keeps the optional sherpa feature compatible with this repository's Rust 1.88
toolchain without changing the application MSRV or suppressing Cargo's MSRV
checks.

Refresh this vendored patch against the upstream crate when upgrading
sherpa-onnx. Preserve the Apache-2.0 license and this change record.
