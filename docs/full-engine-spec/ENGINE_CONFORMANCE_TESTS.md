# Engine Conformance Tests (Stub)

## 1. Purpose
Define a stable, API-visible conformance contract for any core engine implementation that claims compatibility with:
- `docs/SPEC_v0.md`
- `docs/ENGINE_REQUIREMENTS.md`
- `docs/ENGINE_API.md`

This document is intentionally small and incremental. It establishes IDs, pass criteria, and initial invariant coverage so conformance can be expanded without redefining structure.

## 2. Conformance Verdict
An engine is `v0-conformant` when all of the following hold:
- all mandatory API behavior tests pass,
- all mandatory API-visible invariants pass,
- no mandatory test is skipped without an approved waiver record.

Conformance scope is behavior-only. Internal algorithm choices are non-normative unless externally observable.

## 3. API-Visible Invariants (Initial Set)

### INV-EPOCH-001: Epoch Ordering
Always: `stabilized_epoch <= committed_epoch`.

### INV-EPOCH-002: Epoch Monotonicity
Across successful API calls, `committed_epoch` and `stabilized_epoch` are non-decreasing.

### INV-CELL-001: Stale Flag Definition
For `dvc_cell_get_state`, `stale == 1` iff `value_epoch < committed_epoch`.

### INV-DET-001: Replay Determinism
For identical initial state and identical API-call sequence, observable outputs are identical.

### INV-STR-001: Rejected Structural Atomicity
For valid-but-rejected structural ops:
- no partial mutation,
- no epoch increment,
- reject diagnostics are populated.

### INV-CYCLE-001: Non-Iterative Circular Behavior
With iteration disabled:
- circularity does not force `dvc_recalculate` failure by itself,
- circular paths use non-iterative fallback semantics,
- at least one non-fatal circular-reference diagnostic is emitted when change tracking is enabled.

## 4. Initial Test Case Registry (Stub)
- `CT-EPOCH-001`: verify INV-EPOCH-001.
- `CT-EPOCH-002`: verify INV-EPOCH-002.
- `CT-CELL-001`: verify INV-CELL-001.
- `CT-DET-001`: verify INV-DET-001 with deterministic replay script.
- `CT-STR-001`: verify INV-STR-001 for row/col structural rejection.
- `CT-CYCLE-001`: verify INV-CYCLE-001 including diagnostic emission.
- `CT-ENT-051`: verify non-consuming iterator probe/retry semantics for name/control/chart iterators.
- `CT-UDF-035` (planned): verify UDF lifecycle and execution behavior (`register_udf`/`unregister_udf`, invocation, invalidation, volatility handling).

## 5. Harness Shape (Planned)
- Shared scenario corpus (JSON/script form) with expected API-level observations.
- Per-engine adapters:
  - Rust backend adapter,
  - .NET backend adapter.
- One unified report format with per-case `pass/fail/waived`.

## 6. Source Stub Link
Initial source-level stub markers live in:
- `crates/dnavisicalc-core/tests/conformance_invariants_stub.rs`

That file is intentionally minimal and serves as a reminder seam until a cross-engine harness lands.
