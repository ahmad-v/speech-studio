# Testing

Rust unit tests and synthetic fixtures are provided, but no Rust tests were executed in the authoring environment. Validate with cargo test --locked and native CI. Test malformed input and independent reader round trips. For GUI verify keyboard traversal, screen readers, high contrast, scaling, responsive processing and cancellation. Test with network disabled; inspect outbound calls and private-runtime resolution. No screenshot or claim of visual fidelity until a real app runs.

See tests/fixtures/README.md for source and expected outcomes; unit tests use temporary directories. Compare exported files with independent readers and confirm original bytes are unchanged.

All source and test execution must be recorded with commit, target, command, exit status and log URL. Review security risks in docs/STATUS.md before opening hostile files. Do not distribute untested packages.
