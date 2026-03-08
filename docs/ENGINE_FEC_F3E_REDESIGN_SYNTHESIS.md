# FEC/F3E Redesign Synthesis Notes (b4)

## 1. Inputs
- `docs/review/dec_f3e_plan_b/OXFUNC_REVIEW.md`
- `docs/review/dec_f3e_plan_b/FOUNDATION_REVIEW.md`

## 2. b4 Decisions Applied
1. Stable ID contract in seam payloads:
   - introduced `FecNameId`, `FecRangeId`, formula stable IDs.
2. Commit deltas split by concern:
   - `value_delta`, `shape_delta`, `topology_delta`.
3. Spill contract promoted to explicit events:
   - `SpillTakeover`, `SpillClearance`, `SpillBlocked`.
4. Trace schema versioning + field validation:
   - `trace_version=fec-f3e-trace/b4`, `schema_valid`.
5. Name-delta incremental routing enabled:
   - dirty-name updates can use incremental path; full-recalc policy is not forced by FEC.

## 3. Name Dependency Issue (Updated)
Previous b3 gap:
- seam captured name deltas but scheduler lane still treated name edits conservatively/full.

b4 state:
- name dependency identity is now `FecNameId`,
- runtime name-id dependency deltas are consumed by incremental dirty closure.

Limit that remains:
- concurrency hardening and contention replay are still pending.

## 4. Evidence
- `artifacts/fec_f3e/seam_trace.log`
- `artifacts/fec_f3e/seam_trace.event_counts.tsv`
- `artifacts/fec_f3e/seam_trace.callgraph.edges.csv`
- `artifacts/fec_f3e/seam_trace.callgraph.dot`
- seam scenarios:
  - `seam_name_delta_incremental_routing_skips_unrelated_name_updates`
  - `seam_spill_blocked_and_recovery_flow_updates_dependents`
  - `seam_end_to_end_spill_fail_and_recovery_with_dynamic_extent`
