# FEC/F3E Redesign Observations (b4)

## Current Observations
1. Seam identity is now ID-first.
   - `FecNameId`/`FecRangeId`/formula stable IDs are emitted and consumed in contracts/traces.
2. Delta typing is materially clearer.
   - `value_delta`, `shape_delta`, and `topology_delta` separate value vs structure vs invalidation/topology concerns.
3. Spill flows are explicit objects.
   - takeover, clearance, and blocked states are represented as contract events instead of coarse shape labels.
4. Trace schema is now versioned.
   - trace lines emit `trace_version=fec-f3e-trace/b4` and schema field validation status.
5. Name-delta incremental routing is active.
   - name-only dirty edits can run incremental lane; FEC evidence no longer implies forced full-recalc policy.
6. Snapshot/capability guards from b3 remain stable.
   - coordinator snapshot fence and session-bound capability authority continue to hold.
7. End-to-end spill invalidation behavior is now explicitly exercised.
   - scenario covers blocked spill, shrink recovery, and re-block on re-expand with dependent observer updates.

## Remaining Gaps
1. Coordinator storage/execution is still single-thread oriented.
2. Contention/replay stress harness for interleaved commits is still missing.
3. Callgraph extraction is still adjacency-based; causal attribution remains a follow-up.

## Evidence References
- `artifacts/fec_f3e/seam_trace.log`
- `artifacts/fec_f3e/seam_trace.event_counts.tsv`
- `artifacts/fec_f3e/seam_trace.callgraph.edges.csv`
- `artifacts/fec_f3e/seam_trace.callgraph.dot`
- `docs/ENGINE_FEC_F3E_REDESIGN_SPEC.md`
