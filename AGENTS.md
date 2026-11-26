# Speech Studio agent instructions

Read MASTER_SPECIFICATION.md or its repository-specific canonical copy and docs/STATUS.md before changing code. The specification controls; report contradictions instead of silently choosing. Establish all four independent apps and Localwright together. Keep Localwright independently reusable by third-party applications; use the localwright repository and localwright-* Cargo package names consistently. Use Rust-first, Iced native GUI, private runtime only if needed, app-specific heavy engines. No Electron, WebView, Chromium, Tauri, web frontend, mandatory network, telemetry, ads, account, subscription or premium features. No system Python/PATH modifications, global installs, or runtime downloads. No iPad or Intel Mac work. Support five native targets, offline EXE and MSI installers plus portable ZIP for Windows x64/ARM64, Flatpak from start, AUR later, no Snap. All 15 target package combinations per studio require separate evidence. Maintain original files by default and verify real functionality.

Prefer small changes, pinned audited dependencies and exact Localwright revisions. Use standard public GitHub Actions for checks, tests, packaging and release preparation. Use local execution when necessary or cheaper. No paid services, destructive Git actions, unrelated repo access or unauthorized public releases. Stop after repeated failure and report evidence. Never fabricate build/test output, format support, DOI, authorship, signatures or screenshots. Keep statuses and docs current. No secrets in code/logs or untrusted PR execution. Protect release credentials. Report security, license, dependency and platform blockers. All public prose should be natural and avoid em dashes. Citation and donations are voluntary, never part of file output unless requested. Treat user data and imported content as untrusted; do not execute scripts/macros. See master handbook for full acceptance criteria.

GitHub is canonical. GitLab mirrors are deferred until after the first verified releases and explicit approval. Zenodo is the default archival provider; Figshare is an optional alternative. No mirrored credentials or duplicate DOIs.

## Repository scope

Local speech and transcript workbench. Keep domain processing in this repository and shared contracts in Localwright. Do not introduce other studios as dependencies.

Before first CI: format, resolve dependencies, generate and review Cargo.lock, then pin the immutable Localwright source revision in consumer manifests and integration config. See docs/BUILD.md.
