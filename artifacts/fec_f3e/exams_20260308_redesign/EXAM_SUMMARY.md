# FEC/F3E Redesign Examination Summary (2026-03-08)

## Scope
Recorded transactional-seam traces for:
1. `seam_dynamic_reference_retargeting_flows`
2. `seam_spill_takeover_and_clearance_on_referenced_spill_child`

## Artifacts
- `dynamic_retargeting_trace.log`
- `dynamic_retargeting_trace.event_counts.tsv`
- `dynamic_retargeting_trace.callgraph.edges.csv`
- `dynamic_retargeting_trace.callgraph.dot`
- `spill_takeover_clearance_trace.log`
- `spill_takeover_clearance_trace.event_counts.tsv`
- `spill_takeover_clearance_trace.callgraph.edges.csv`
- `spill_takeover_clearance_trace.callgraph.dot`

## Event Volume
- dynamic retargeting flow: `65` boundary events
- spill takeover/clearance flow: `51` boundary events

## Key Observations
1. Dynamic retargeting emits dependency deltas during selector path transitions.
   - `D1` and `D2` commits showed `dep_delta_cells>0` when runtime targets changed.
2. Spill-shape transitions are explicit at commit.
   - `spill_shape_delta=created` when spill expanded.
   - `spill_shape_delta=cleared` when spill shrank away.
3. Transaction statuses were deterministic in these runs.
   - all observed commits were `status=Applied`.
4. Call-graph phase pattern is stable:
   - `fec.open_session -> fec.capability_view -> f3e.execute -> fec.commit -> engine.evaluate_cell_via_f3e`

## Practical Impact
1. Dynamic `INDIRECT`/`OFFSET` targets now participate in runtime dependency tracking, improving incremental invalidation behavior.
2. Spill takeover/clearance transitions are now surfaced as seam metadata rather than implicit side effects.
