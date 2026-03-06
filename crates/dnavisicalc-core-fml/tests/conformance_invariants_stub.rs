//! Conformance registry marker for core crate test discovery.
//!
//! Executable cross-engine conformance coverage is implemented in:
//! `crates/dnavisicalc-engine/tests/conformance_smoke.rs`.
//! This file intentionally avoids ignored TODO tests so conformance backlog
//! tracking remains in docs plus executable suites.

#[test]
fn conformance_registry_marker() {
    assert!(true);
}
