# FEC/F3E Recorded Examinations (2026-03-08)

## Scenarios
1. Dynamic retargeting via `INDIRECT` and `OFFSET`:
   - test: `seam_dynamic_reference_retargeting_flows`
   - trace: `dynamic_retargeting_trace.log`
2. Spill takeover + spill clearance on referenced spill child:
   - test: `seam_spill_takeover_and_clearance_on_referenced_spill_child`
   - trace: `spill_takeover_clearance_trace.log`

## Generated call-graph artifacts
- Dynamic retargeting:
  - `dynamic_retargeting_trace.callgraph.edges.csv`
  - `dynamic_retargeting_trace.callgraph.dot`
  - `dynamic_retargeting_trace.event_counts.tsv`
- Spill takeover/clearance:
  - `spill_takeover_clearance_trace.callgraph.edges.csv`
  - `spill_takeover_clearance_trace.callgraph.dot`
  - `spill_takeover_clearance_trace.event_counts.tsv`

## Key observations
- Dynamic retargeting flow showed selector-driven re-evaluation behavior:
  - `INDIRECT(IF(...))` and `OFFSET(anchor,selector,...)` re-target correctly when selector cells change.
  - direct edits to runtime-target cells (`A2`, `B2`) did not trigger recalculation until selector/anchor paths re-invalidated the formula.
- Spill takeover/clearance flow showed full lifecycle propagation:
  - expanding `SEQUENCE` to include `A2` changed a dependent reference (`B1`) from blank-path to spill value path.
  - shrinking `SEQUENCE` removed spill ownership from `A2`, and the dependent reference returned to blank-path behavior.
- In both traces, core eval call path remained:
  - `fec.capability_view -> f3e.evaluate -> fec.publish_result -> engine.evaluate_cell_via_f3e`
