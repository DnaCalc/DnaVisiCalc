# Rust Engine Implementations

This directory is for spec-derived Rust core engine implementations:
- `engines/rust/<implementation-id>/`

The in-workspace Rust reference backend used by app crates is:
- `crates/dnavisicalc-coreengine-rust/`

Do not mix the two roles:
- reference backend crate supports day-to-day repository integration,
- spec-derived implementations under `engines/rust/` are run-managed artifacts governed by `docs/OPERATIONS.md`.
