# Engine Formal Properties (API-Visible)

## 1. Purpose
Define property forms and property IDs used to tighten the engine contract toward formal verification, while remaining implementation-independent and API-visible.

This document is normative for:
- property naming,
- temporal/safety interpretation,
- trace-level observability requirements for conformance runs.

## 2. Property Forms

### 2.1 Safety (`INV-*`)
State predicates that must always hold on every observable state in a run.

Form:
- `Always P(state_n)`

Examples:
- epoch ordering,
- stale-flag consistency,
- deterministic replay equivalence.

### 2.2 Temporal (`TEMP-*`)
Order-sensitive properties over API-call traces.

Canonical forms:
- `AlwaysAfter(trigger, Eventually condition)`:
  after trigger event, condition must become true in finite steps.
- `AlwaysAfter(trigger, Never condition)`:
  after trigger event, forbidden condition must never become true until a specified release condition.
- `AlwaysAfter(trigger, condition Until release)`:
  condition must hold continuously until release.

Temporal scope is always finite-trace testable (no dependence on wall-clock background behavior).

## 3. Observable Trace Model
Each conformance run emits an abstract trace:
- API call (`fn`, args class, status),
- diagnostic outputs,
- selected state snapshots (epochs, cell states, iterators, outputs).

Properties may only depend on this observable trace model, not hidden internal state.

## 4. v0 Property Set

### 4.1 Safety Properties
- `INV-EPOCH-001`: `Always(stabilized_epoch <= committed_epoch)`.
- `INV-EPOCH-002`: `Always(committed/stabilized epochs are non-decreasing across successful calls)`.
- `INV-CELL-001`: `Always(cell.stale == (cell.value_epoch < committed_epoch))`.
- `INV-DET-001`: replay-equivalent traces produce replay-equivalent observable outputs.
- `INV-STR-001`: rejected structural calls are atomic no-ops with reject diagnostics.
- `INV-CYCLE-001`: non-iterative cycle handling is non-fatal and diagnostically observable.

### 4.2 Temporal Properties
- `TEMP-RECALC-001`:
  - Trigger: explicit recalc call in manual mode on dirty graph.
  - Requirement: affected values eventually stabilize for current committed epoch.
- `TEMP-STREAM-001`:
  - Trigger: stream formula exists and no `tick_streams` is called.
  - Requirement: stream-exposed values never advance.
- `TEMP-STREAM-002`:
  - Trigger: `tick_streams` call.
  - Requirement: stream-dependent outputs eventually reflect new stream counter (subject to recalc mode).
- `TEMP-REJECT-001`:
  - Trigger: valid-but-rejected structural op.
  - Requirement: no observable state mutation is ever attributable to the rejected call.
- `TEMP-VOL-001`:
  - Trigger: volatile formulas present, no explicit mutation/invalidation/recalc event.
  - Requirement: volatile outputs never change.

## 5. Toward Formalization
These properties are intended to map directly to:
- Lean theorem statements for pure safety predicates,
- TLA+/trace-checker predicates for temporal properties,
- executable conformance checks over recorded traces.

The intended progression is:
1. executable property checks in conformance harness,
2. machine-readable trace schema freeze,
3. formal proof/model-check alignment against the same property IDs.
