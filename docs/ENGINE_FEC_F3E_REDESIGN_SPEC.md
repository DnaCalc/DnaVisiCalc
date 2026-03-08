# FEC/F3E Redesign Specification (Plan B)

## 1. Purpose
This is the active internal seam contract for `dnavisicalc-core-fml` after the Plan B redesign.

It replaces the prior incremental `compile/declare/evaluate` split with a transactional seam:
1. `prepare`
2. `open_session` + `capability_view`
3. `execute`
4. `commit`

Archived pre-redesign specs are preserved under:
- `docs/archive/fec_f3e/pre_redesign_20260308/`

## 2. Scope
In scope:
- internal FEC/F3E boundary in `crates/dnavisicalc-core-fml/src/fec_f3e/`
- engine integration for cell/name formula install and recalculation
- runtime observation capture for dependency updates
- spill-shape transition signaling
- trace evidence hooks for boundary call graphs

Out of scope:
- external/public API changes
- C ABI changes
- non-`core-fml` crate behavior changes

## 3. Core Types
The redesign contract is defined in:
- `crates/dnavisicalc-core-fml/src/fec_f3e/contracts.rs`

Primary types:
- `FormulaPlan`
- `EvalRequest`
- `EvalTransaction`
- `CommitResult`
- `EvalObservation`
- `F3eObservedDependencies`
- `F3eDependencyDelta`
- `SpillShapeDelta`
- `CommitStatus`

## 4. Transaction Phases

### 4.1 `prepare` (F3E)
`prepare` MUST:
1. parse and bind formula text
2. compute static dependencies
3. compute required capabilities
4. classify dependency profile
5. produce deterministic `FormulaToken`

`prepare` MUST NOT mutate engine or host state.

### 4.2 `install_plan` (FEC)
`install_plan` MUST register a formula plan for a `FecFormulaId`.

When replacing an existing plan for the same formula id:
1. previous observed dependency state is retained until next `commit`
2. previous spill-shape state is retained until next `commit`

This allows explicit delta signaling on formula edits.

### 4.3 `open_session` + `capability_view` (FEC)
`open_session` MUST create an explicit session id bound to:
- formula id
- expected token
- snapshot epoch

`capability_view` MUST return a deterministic decision:
- `Allowed`
- `Denied(<capability>)`

### 4.4 `execute` (F3E)
`execute` MUST return an `EvalTransaction` containing:
- runtime value
- result kind
- typed runtime observations (`EvalObservation`)
- capability decision used by execution

If capability is denied, `execute` MUST produce a deterministic error runtime.

### 4.5 `commit` (FEC)
`commit` MUST be the single point where value/dependency/shape publication is accepted or rejected.

`commit` MUST return one explicit status:
- `Applied`
- `RejectedTokenMismatch`
- `RejectedCapabilityDenied`
- `RejectedSnapshotConflict`

On `Applied`, `commit` MUST emit:
- scalar value for engine publication
- dependency delta
- spill-shape delta
- post-commit token

## 5. Runtime Observation Semantics
Observed events currently include:
- `ReadCell`
- `ReadName`
- `ReadSpillChild`
- `VolatileRead`
- `ExternalRead`

Observed dependencies are normalized into:
- `cells_read`
- `names_read`
- `spill_children_read`

`F3eDependencyDelta` is computed by set-diff against previous observed state.

## 6. Token, Session, Snapshot Rules
1. `commit` MUST reject when session is missing or bound to another formula id.
2. `commit` MUST reject when transaction token mismatches coordinator token.
3. `commit` MUST reject when transaction snapshot epoch mismatches session snapshot epoch.
4. On applied dependency/spill-shape change, token MUST rotate.
5. On no semantic change, token MAY remain stable.

## 7. Spill-Shape Rules
Spill-shape deltas are recognized as:
- `None`
- `Created`
- `Resized`
- `Cleared`

For cell formulas:
1. `ArraySpill` runtime maps to an anchor range.
2. previous/next range diff determines shape delta.

For non-cell formula ids (names), spill-shape delta is always `None`.

## 8. Engine Integration Rules
Engine integration in `crates/dnavisicalc-core-fml/src/engine.rs` MUST:
1. route formula install through `prepare` + `install_plan`
2. route evaluation through `open_session` + `capability_view` + `execute` + `commit`
3. apply commit dependency deltas to runtime reverse-dependency state
4. include runtime reverse dependencies in dirty-closure traversal
5. trigger a full-recalc fallback when spill-shape change is observed during incremental recalc

## 9. Trace Contract
Opt-in trace flag:
- `DNAVISICALC_FEC_F3E_TRACE=1`

Redesign events include:
- `f3e.prepare`
- `f3e.execute`
- `fec.install_plan`
- `fec.open_session`
- `fec.capability_view`
- `fec.commit`
- engine boundary wrappers (`engine.set_formula`, `engine.evaluate_cell_via_f3e`, recalc events)

Trace events MUST remain one-line and machine-parseable.

## 10. Conformance Scenarios (Current)
Primary seam scenario lane:
- `crates/dnavisicalc-core-fml/tests/fec_f3e_seam_scenarios_tests.rs`

High-pressure flows:
1. dynamic retargeting (`INDIRECT` / `OFFSET`) with calc-time target updates
2. spill takeover and spill-clearance for externally referenced spill children

## 11. Evidence Artifacts (Current Redesign Pass)
Redesign examination artifacts:
- `artifacts/fec_f3e/exams_20260308_redesign/`

Archive of previous seam outputs:
- `artifacts/fec_f3e/archive/pre_redesign_20260308/`

## 12. Open Items
1. capability-denial profile policy (beyond deterministic error)
2. name-level dependency-driven incremental invalidation strategy
3. cross-epoch replay schema for transactional seam traces
