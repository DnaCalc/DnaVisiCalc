# FEC/F3E Redesign Specification (Plan B, b4)

## 1. Purpose
Active internal FEC/F3E seam contract for `dnavisicalc-core-fml`.

Transaction lane:
1. `prepare`
2. `open_session` + `capability_view`
3. `execute`
4. `commit`

## 2. Version and Scope
- Interface marker: `fec-f3e-redesign/b4` (`src/fec_f3e/spec.rs`).
- Trace schema marker: `fec-f3e-trace/b4` (`src/fec_f3e/trace.rs`).
- In scope: internal seam semantics, engine integration policy, test/evidence lane.

## 3. Identity Contract (b4)
Stable IDs are first-class seam identity:
- `FecFormulaId` with `stable_id()`.
- `FecNameId` for name dependency identity.
- `FecRangeId` for spill-range identity.

Names are metadata in traces (`name_label`) but no longer contractual dependency identity strings.

## 4. Commit Delta Contract (b4)
`CommitResult` now separates mutation classes:
- `value_delta: FecValueDelta`
- `shape_delta: FecShapeDelta`
- `topology_delta: FecTopologyDelta`

`topology_delta` carries:
- dependency set deltas (`cells`, `name_ids`, `spill_children`),
- impacted cell/name-id sets,
- typed impact class (`None|DependencySetChanged|SpillRangeChanged|SpillBlocked`).

## 5. Spill Event Contract (b4)
Spill transitions are explicit event objects in `shape_delta.spill_event`:
- `SpillTakeover`
- `SpillClearance`
- `SpillBlocked`

Each includes affected range/anchor context and invalidation scope.
Engine spill scheduler hints are derived from these events (FEC emits metadata; policy remains engine-owned).

## 6. Snapshot and Capability Semantics
- Sessions bind `(formula_id, expected_token?, snapshot_epoch)`.
- `commit` enforces:
  - tx/session snapshot equality,
  - coordinator snapshot fence equality.
- Capability decisions are session-bound during `capability_view` and commit-validated.
- Reject outcomes remain machine-typed (`CommitStatus` + `CommitRejectDetail`).

## 7. Incremental Policy Boundary
FEC provides topology evidence; engine owns recalc policy.

b4 engine integration now consumes runtime name-id deltas in incremental routing:
- dirty-name updates are eligible for incremental recalc,
- no automatic “name edit => full recalc” at seam policy boundary.

This keeps full-recalc policy above FEC/F3E, consistent with core-engine ownership.

## 8. Trace Schema Contract (b4)
When `DNAVISICALC_FEC_F3E_TRACE=1`:
- each event includes `trace_version=fec-f3e-trace/b4`,
- trace emitter validates field keys (non-empty, unique) and emits `schema_valid`.

Key b4 commit fields:
- `formula_stable_id`
- `shape_delta`
- `topology_impact`
- `value_changed`
- dependency delta counts
- snapshot/capability reject context

## 9. Perf Scaffolding (b4)
`FecSeamPerfCounters` now includes explicit spill-event counters:
- `spill_takeover_count`
- `spill_clearance_count`
- `spill_blocked_count`

Engine API remains:
- `fec_seam_perf_snapshot()`
- `reset_fec_seam_perf_counters()`

## 10. Validation Lane
Primary seam scenario suite:
- `crates/dnavisicalc-core-fml/tests/fec_f3e_seam_scenarios_tests.rs`

b4 targeted scenarios include:
- name-id incremental routing on dirty-name edits,
- spill blocked -> recovery flow,
- end-to-end spill fail/recovery/re-fail with dependent observer updates,
- external scheduler handoff and conservative fallback behavior.

Foundation handoff prompt:
- `docs/ENGINE_FEC_F3E_FOUNDATION_UPDATED_SPEC_POINTERS_PROMPT.md`

## 11. Open Items
1. Threaded coordinator implementation (`Arc`/sync primitives) for concurrent scheduler pilots.
2. Deterministic contention replay pack for commit conflict interleavings.
3. Higher-fidelity causal callgraph derivation (current graph is adjacency-derived).
