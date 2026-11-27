# Packaging preparation

The target matrix retains all 15 combinations. The installable formats in this matrix still require device validation. This is an unsigned development payload. The experimental driver checks native host and executable architecture, requires Cargo.lock and refuses packaging until the maintainer approves the candidate application identity in config.json. It does not itself prove that a lockfile, dependencies or output are safe.

| Recipe | Status |
|---|---|
| Windows portable ZIP x64/ARM64 | Driver present; unverified |
| Windows MSI x64/ARM64 | WiX source candidate; unverified |
| Windows offline setup EXE x64/ARM64 | Burn template only; native tool validation blocked |
| Linux DEB/RPM x64/ARM64 | Driver present; dependencies and distro QA pending |
| Linux AppImage x64/ARM64 | Driver present; reviewed pinned tool/runtime and ABI testing required |
| Linux Flatpak x64/ARM64 | Manifest template; vendor source and pinned runtime required |
| macOS ARM64 DMG | APP/DMG driver; ad hoc signing, not notarized, unverified |

Baseline workflow calls ZIP, DEB or DMG after locked tests and build. It intentionally stops on unapproved identity. RPM, WiX and AppImage tooling need separate audited provisioning and jobs. No release job is provided: release preparation cannot proceed before all required verification and explicit approval. Flatpak template omits network permission and grants no blanket filesystem access; portal integration is missing. No private engines or models included in baseline payload. Speech baseline cannot claim transcription. An offline installer must include everything advertised, and these recipes are not release ready.

Windows MSI and EXE must use the same product identity and payload, separately verify repair/upgrade/uninstall and architecture, and embed all payloads. Candidate application ID `org.localwright.SpeechStudio` is not a verified owner namespace or trademark. Product GUID generated deterministically as a proposed identifier, not an existing registered product. Review before fixing identity. Linux and macOS require real-device validation. No signing keys, service accounts or secrets are included.

## Optional speech model pack

The [private Whisper model pack](../docs/WHISPER_MODEL_PACK.md) can be assembled offline from the pinned local model and provisioned beside an ASR-enabled binary.

## Portable CI previews

The check workflow builds an unsigned portable archive on each native target, extracts it, runs CLI help from the extracted payload, and uploads it with SHA-256 checksums. Download it from a successful job under GitHub Actions artifacts. Launch the executable from the extracted folder. These archives are intended for evaluation. Linux may require system libraries, and macOS Gatekeeper may block an unsigned executable.
