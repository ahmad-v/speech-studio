# Third-party notices (incomplete)

No third-party engine binaries, models, icons or fonts are redistributed in this source snapshot. Cargo dependencies are declared in manifests. Their full transitive inventory, licenses, notices, advisories and redistribution obligations are unresolved until dependency resolution and review. Do not distribute binaries before completing these checks. See docs/DEPENDENCIES.md.

The source tree vendors `whisper-rs-sys 0.13.1` to select a Windows/MSVC ABI binding for Windows ARM64. Preserve `vendor/whisper-rs-sys/LICENSE` (Unlicense) and `vendor/whisper-rs-sys/whisper.cpp/LICENSE` (MIT) with redistributed source or binaries. The vendored tree contains source code, not a Whisper executable or model. The generated ARM64 binding provenance and SHA-256 are recorded in `vendor/whisper-rs-sys/BINDINGS_PROVENANCE.md`.

The optional `tts` Cargo feature adds `sherpa-onnx 2.0.3` (Apache-2.0) and `piper-plus-g2p 0.5.4` (MIT). The source snapshot does not include the voice model. The upstream voice model card identifies the training dataset as public domain but does not state model-weight redistribution terms; do not redistribute the model until those terms are confirmed. The static native sherpa archives and their complete licenses/notices must also be reviewed for each target before shipping a TTS-enabled binary.

Portable CI previews include `DEPENDENCY_LICENSES.json` and collected Cargo dependency license and notice texts under `third-party/`. The inventory includes normal and build dependency edges. It is not a binary linkage audit, a choice among upstream license alternatives, or a review of separately embedded assets and platform libraries. Empty notice lists require follow-up against the upstream source.
