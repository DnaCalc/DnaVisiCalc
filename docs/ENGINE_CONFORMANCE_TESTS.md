# Engine Conformance Tests

## 1. Purpose
Define a stable, API-visible conformance contract for any core engine implementation that claims compatibility with:
- `docs/SPEC_v0.md`
- `docs/ENGINE_REQUIREMENTS.md`
- `docs/ENGINE_API.md`

Conformance is behavior-only. Internal architecture (dependency graph shape, incremental strategy, scheduler internals) is non-normative unless externally observable.

## 2. Conformance Verdict
An engine is `v0-conformant` when all of the following hold:
- all mandatory behavior cases pass,
- all mandatory safety invariants pass,
- all mandatory temporal properties pass,
- no mandatory case is skipped without an approved waiver entry.

## 3. Systematic Expansion Approach

### 3.1 Build-Out Phases
1. `Phase A` harness stability:
   - deterministic runner,
   - per-engine adapter seams,
   - machine-readable result report.
2. `Phase B` safety invariants:
   - epoch/state/error-model invariants,
   - deterministic replay invariants.
3. `Phase C` feature-surface cases:
   - API-by-API behavior coverage,
   - structural rewrite and dynamic-array coverage.
4. `Phase D` temporal/liveness properties:
   - eventually/never/always-after rules encoded as executable checks.
5. `Phase E` conformance-performance coupling:
   - non-gating performance signatures recorded alongside conformance runs.

### 3.2 Case Authoring Workflow
For each new requirement:
1. Select requirement IDs from `ENGINE_REQUIREMENTS.md`.
2. Add/extend property IDs in `ENGINE_FORMAL_PROPERTIES.md`.
3. Add conformance case IDs (`CT-*`) in this doc.
4. Add executable test coverage (shared harness first; engine-local tests only as stopgaps).
5. Record outcome in machine-readable report (`pass` / `fail` / `waived`) with engine build identity.

## 4. Property Registry (v0 Mandatory Core)

### 4.1 Safety Invariants (`INV-*`)
- `INV-EPOCH-001`: Always `stabilized_epoch <= committed_epoch`.
- `INV-EPOCH-002`: Across successful calls, `committed_epoch` and `stabilized_epoch` never decrease.
- `INV-CELL-001`: For cell state, `stale == (value_epoch < committed_epoch)`.
- `INV-DET-001`: Identical initial state + identical API-call sequence yields identical observable outputs.
- `INV-STR-001`: Valid-but-rejected structural ops are atomic no-ops (no partial mutation, no epoch increment, reject diagnostics present).
- `INV-CYCLE-001`: With iteration disabled, circularity is non-fatal for recalc and emits at least one diagnostic when diagnostics are enabled.

### 4.2 Temporal Properties (`TEMP-*`)
- `TEMP-RECALC-001`: In manual mode, after explicit recalc, affected dirty values eventually stabilize for the current committed epoch.
- `TEMP-STREAM-001`: Without `tick_streams`, stream counters never advance.
- `TEMP-STREAM-002`: After `tick_streams` (and recalc when required), stream-dependent outputs eventually reflect the advanced stream counter.
- `TEMP-REJECT-001`: After rejected structural op, affected observable state never changes as a side effect of that rejected call.
- `TEMP-VOL-001`: Volatile functions do not autonomously self-tick; value changes require explicit mutation/invalidation/recalc triggers.

Temporal property semantics are defined in `docs/ENGINE_FORMAL_PROPERTIES.md`.

## 5. Case Registry

### 5.1 Mandatory Cases (Current Core)
- `CT-EPOCH-001`: verify `INV-EPOCH-001`.
- `CT-EPOCH-002`: verify `INV-EPOCH-002`.
- `CT-CELL-001`: verify `INV-CELL-001`.
- `CT-DET-001`: verify `INV-DET-001` with deterministic replay script.
- `CT-STR-001`: verify `INV-STR-001`.
- `CT-CYCLE-001`: verify `INV-CYCLE-001`.
- `CT-TEMP-RECALC-001`: verify `TEMP-RECALC-001`.
- `CT-TEMP-STREAM-001`: verify `TEMP-STREAM-001`.
- `CT-TEMP-STREAM-002`: verify `TEMP-STREAM-002`.
- `CT-TEMP-REJECT-001`: verify `TEMP-REJECT-001`.
- `CT-TEMP-VOL-001`: verify `TEMP-VOL-001`.

### 5.2 Near-Term Feature Coverage Targets
- `CT-STR-010`: mixed/absolute reference rewrite matrix under row/col insert/delete.
- `CT-ARR-020`: dynamic-array spill anchor/range behavior under structural ops.
- `CT-FN-030`: function-set parity corpus (`REQ-CALC-008`) across both engines.
- `CT-ERR-040`: reject/error diagnostic surfaces (`dvc_last_*`) under malformed/invalid/rejected operations.
- `CT-ENT-050`: controls/charts define/query/iterate/remove and persistence roundtrip.

## 6. Harness and Reporting Shape
- Shared scenario corpus (`runs/conformance/scenarios/*`) with stable IDs.
- Per-engine execution via C API loader configuration (`DNAVISICALC_COREENGINE*`).
- Unified report artifact (`runs/conformance/reports/*.json`) containing:
  - engine identity (backend id, DLL path/hash, build metadata),
  - case outcomes,
  - waived-case list with rationale,
  - optional performance counters.

## 7. Initial Implemented Coverage
- Current executable conformance smoke tests:
  - `crates/dnavisicalc-engine/tests/conformance_smoke.rs`
- Core-side registry marker only:
  - `crates/dnavisicalc-core/tests/conformance_invariants_stub.rs` (no ignored `CT-*` backlog tests)
