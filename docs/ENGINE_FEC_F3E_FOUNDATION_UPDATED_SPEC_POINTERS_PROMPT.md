# FEC/F3E Updated Spec Pointers Prompt (Foundation Handoff)

Use the prompt below when requesting Foundation review and integration planning.

```text
Please pull the latest FEC/F3E redesign (Plan B, b4) material from DnaVisiCalc and evaluate it as candidate seam input for core-engine design.

Primary spec and synthesis:
- docs/ENGINE_FEC_F3E_REDESIGN_SPEC.md
- docs/ENGINE_FEC_F3E_REDESIGN_SYNTHESIS.md
- docs/ENGINE_FEC_F3E_REDESIGN_OBSERVATIONS.md

Review inputs addressed:
- docs/review/dec_f3e_plan_b/FOUNDATION_REVIEW.md
- docs/review/dec_f3e_plan_b/OXFUNC_REVIEW.md

Executable seam evidence:
- crates/dnavisicalc-core-fml/tests/fec_f3e_seam_scenarios_tests.rs
- artifacts/fec_f3e/seam_trace.log
- artifacts/fec_f3e/seam_trace.event_counts.tsv
- artifacts/fec_f3e/seam_trace.callgraph.edges.csv
- artifacts/fec_f3e/seam_trace.callgraph.dot

Key implementation surfaces:
- crates/dnavisicalc-core-fml/src/fec_f3e/contracts.rs
- crates/dnavisicalc-core-fml/src/fec_f3e/fec_host.rs
- crates/dnavisicalc-core-fml/src/fec_f3e/f3e_engine.rs
- crates/dnavisicalc-core-fml/src/fec_f3e/trace.rs
- crates/dnavisicalc-core-fml/src/engine.rs

Requested evaluation focus:
1) Contract quality:
   - stable id contract (`FecNameId`, `FecRangeId`, formula stable id),
   - split commit deltas (`value_delta`, `shape_delta`, `topology_delta`),
   - explicit spill events (`SpillTakeover`, `SpillClearance`, `SpillBlocked`).
2) Policy boundary:
   - FEC emits evidence; core engine owns recalc policy and spill optimization decisions.
3) Scheduler readiness:
   - whether current topology and spill metadata is sufficient for higher-level incremental/full-recalc selection.
4) Traceability:
   - whether b4 trace schema (`trace_version=fec-f3e-trace/b4`) is adequate for diagnosis and replay tooling.
5) Remaining gaps:
   - concurrent coordinator model and contention replay harness scope.

New end-to-end spill scenario to examine:
- seam_end_to_end_spill_fail_and_recovery_with_dynamic_extent
  (spill blocked by occupied cell, recovery on shrink, re-block on re-expand, observer dependency behavior included).

Please return:
- accept / conditionally accept / reject for core design input,
- required contract adjustments (if any),
- missing evidence required before integration.
```
