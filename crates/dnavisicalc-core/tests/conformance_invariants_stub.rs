//! Conformance-invariant scaffold.
//!
//! This file intentionally contains minimal stubs so invariant IDs from
//! `docs/ENGINE_CONFORMANCE_TESTS.md` are tracked in source control and can be
//! expanded into a cross-engine harness later.

#[test]
fn conformance_stub_registry_exists() {
    // Placeholder smoke check so this file is always compiled/run.
    assert!(true);
}

#[test]
#[ignore = "TODO(CT-EPOCH-001): implement shared conformance harness"]
fn inv_epoch_001_epoch_ordering() {
    // INV-EPOCH-001
}

#[test]
#[ignore = "TODO(CT-EPOCH-002): implement shared conformance harness"]
fn inv_epoch_002_epoch_monotonicity() {
    // INV-EPOCH-002
}

#[test]
#[ignore = "TODO(CT-CELL-001): implement shared conformance harness"]
fn inv_cell_001_stale_flag_definition() {
    // INV-CELL-001
}

#[test]
#[ignore = "TODO(CT-DET-001): implement deterministic replay harness"]
fn inv_det_001_replay_determinism() {
    // INV-DET-001
}

#[test]
#[ignore = "TODO(CT-STR-001): implement structural-reject invariant checks"]
fn inv_str_001_rejected_structural_atomicity() {
    // INV-STR-001
}

#[test]
#[ignore = "TODO(CT-CYCLE-001): implement cycle diagnostic conformance checks"]
fn inv_cycle_001_non_iterative_cycle_signal() {
    // INV-CYCLE-001
}
