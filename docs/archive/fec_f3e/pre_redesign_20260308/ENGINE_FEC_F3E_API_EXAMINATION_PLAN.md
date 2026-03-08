# FEC/F3E API Examination Plan

## Objective
Validate and evolve the internal `FEC/F3E` seam in `dnavisicalc-core-fml` so that:
- current v0 behavior remains stable,
- the boundary supports future dynamic dependency semantics,
- we have concrete evidence from instrumentation and targeted runs.

## Scope
- In scope:
  - boundary tracing and observability,
  - focused seam test matrix and scenario runs,
  - API pressure-point analysis,
  - one minimal proving increment after analysis.
- Out of scope:
  - broad evaluator rewrite,
  - external API contract changes,
  - full Excel-parity feature expansion.

## Workstream 1: Baseline and Questions
1. Record the current contract shape and open questions:
   - static-only dependency declaration vs dynamic updates,
   - token lifecycle (`prior_token` usage),
   - capability enforcement policy,
   - publish-result envelope requirements.
2. Keep contract/source anchors current:
   - `crates/dnavisicalc-core-fml/src/fec_f3e/contracts.rs`
   - `crates/dnavisicalc-core-fml/src/fec_f3e/spec.rs`
   - `docs/ENGINE_FEC_F3E_FIRST_PASS.md`

## Workstream 2: Instrumentation (Opt-In)
Add lightweight, opt-in tracing (for example `DNAVISICALC_FEC_F3E_TRACE=1`) with one-line events.

### Boundary Hooks
- `Engine::set_formula`
- `Engine::recalculate_full`
- `Engine::recalculate_incremental`
- `Engine::evaluate_cell_via_f3e`
- `Engine::evaluate_name_via_f3e`
- `F3eEngine::compile`
- `F3eEngine::declare_dependencies`
- `F3eEngine::evaluate`
- `FecHost::capability_view`
- `FecHost::register_dependencies`
- `FecHost::publish_result`

### Event Fields
- `formula_id` / `target`
- `dep_count`
- `required_caps`
- `dependency_profile`
- `token` (when present)
- `recalc_mode` (`full`/`incremental`)
- `duration_us`
- `result_kind` (scalar/array/error)

### Performance Guardrails
- Tracing disabled by default.
- Optional sampling for deep eval traces to avoid hot-path distortion.

## Workstream 3: Seam Run Matrix
Run targeted scenarios that pressure different boundary semantics.

1. Static no-dependency:
   - `A1 := =2+2`
2. Static ref-only:
   - `A1 := 1`, `B1 := =A1+1`
3. Name-path chain:
   - names feeding names, then into a cell
4. Branch behavior:
   - `IF` with condition flips
5. Dynamic-intent reference functions:
   - `INDIRECT`, `OFFSET`
6. Spill/shape-driven behavior:
   - `SEQUENCE` producer + `#` consumer
7. Volatile/external intent:
   - `NOW`, `RAND`, `STREAM`
8. Structural edits:
   - insert/delete row/column after formula installation
9. Recalc modes:
   - automatic vs manual
10. Incremental path:
   - dirty closure updates vs forced full recalc

## Workstream 4: Evidence Capture
For each scenario:
1. Capture trace log with boundary events.
2. Summarize:
   - declared dependencies/capabilities,
   - actual evaluation path characteristics,
   - token behavior.
3. Mark mismatch classes:
   - declaration too broad,
   - declaration too narrow,
   - runtime-dependency shift not representable.

## Workstream 5: API Pressure Analysis
Derive concrete findings:
1. Where `declare_dependencies` is currently redundant.
2. Where `evaluate` needs dependency-delta or shape-change signaling.
3. Where FEC contracts need stronger semantics:
   - capability denial policy,
   - token update behavior,
   - publish-result envelope evolution.

## Workstream 6: API vNext Options
Document candidate revisions and tradeoffs:
1. Keep three-call model; add optional dependency updates from `evaluate`.
2. Split static declaration and observed/runtime declaration channels.
3. Token-diff oriented registration/update model.

For each option include:
- compatibility impact,
- implementation effort,
- test impact,
- migration path.

## Workstream 7: One Proving Increment
Implement one narrow improvement to validate chosen direction (recommended first target: spill/shape dependency update signal), with tests and trace evidence.

## Suggested Run Set
1. `cargo test -p dnavisicalc-core-fml`
2. Focused seam tests (new targeted coverage for the matrix above)
3. Scenario run with tracing enabled
4. Sanity perf check tracing off vs on

## Execution Snapshot (2026-03-07)
Status:
- Workstream 2 (instrumentation): completed for all listed hooks.
- Workstream 3 (seam run matrix): covered by targeted scenario tests.
- Workstream 4/5 (evidence + pressure analysis): captured and summarized below.
- Workstream 7 (one proving increment): pending follow-up, not included in this pass.

### Runs Executed
1. `cargo test -p dnavisicalc-core-fml`
   - result: pass
2. `cargo test -p dnavisicalc-core-fml --test fec_f3e_seam_scenarios_tests`
   - result: pass (10/10)
3. `DNAVISICALC_FEC_F3E_TRACE=1 cargo test -p dnavisicalc-core-fml --test fec_f3e_seam_scenarios_tests -- --nocapture`
   - result: pass (10/10), trace captured
   - artifact: `artifacts/fec_f3e/seam_trace.log`
4. Trace-off vs trace-on scenario lane runtime check:
   - trace off: `finished in 0.02s`
   - trace on: `finished in 0.02s`

### Trace Evidence Summary
- Captured boundary events: `301`
- Event-type counts:
  - `f3e.evaluate`: `45`
  - `fec.capability_view`: `45`
  - `fec.publish_result`: `45`
  - `engine.evaluate_cell_via_f3e`: `36`
  - `engine.recalculate_full`: `28`
  - `f3e.declare_dependencies`: `21`
  - `fec.register_dependencies`: `21`
  - `f3e.compile`: `20`
  - `engine.set_formula`: `18`
  - `engine.recalculate_incremental`: `11`
  - `engine.evaluate_name_via_f3e`: `9`
  - `engine.set_name_formula`: `2`
- Token lifecycle signal:
  - `prior_token` observed values: `none` only (`21` occurrences)

### Mismatch Classes Observed
1. Declaration too broad:
   - Branch formula `IF(A1>0,A2,A3)` declared `dep_count=3` (`A1`,`A2`,`A3`) even when one branch path is active at runtime.
2. Declaration too narrow:
   - `INDIRECT("A1")` declared `dep_count=0` with `required_caps=reference_resolution`; runtime dependency resolution occurs but is not represented as declared static edges.
3. Runtime dependency/shape shift not representable:
   - Spill producer (`SEQUENCE`) emits `result_kind=array_spill`, while consumer declaration remains static (`A1#` ref dependency) with no explicit shape-delta signal crossing the seam.
4. Token update semantics unexercised:
   - `declare_dependencies` receives no prior token in all traced calls; no registration update/diff path is currently expressible.

### API vNext Decisions (Evidence-Driven)
1. Keep the three-call seam (`compile` / `declare_dependencies` / `evaluate`) for compatibility stability.
   - impact: low, no external API change.
2. Add optional observed-runtime metadata from `evaluate`.
   - minimum fields: observed dependency additions/removals and spill shape-change marker.
   - rationale: needed for `INDIRECT`/`OFFSET` and spill-shape pressure cases.
3. Introduce token-aware declaration updates.
   - wire `prior_token` into declaration flow and support host update semantics (`register_or_update` style behavior).
   - rationale: traces show token plumbing exists but is not currently exercised.
4. Extend publish-result envelope with non-value metadata.
   - minimum fields: `result_kind`, optional spill-shape summary, optional dependency-delta summary.
   - rationale: FEC publication currently loses information needed for downstream incremental policy decisions.

Detailed contract draft (incremental + clean-slate redesign):
- `docs/ENGINE_FEC_F3E_VNEXT_AND_REDESIGN_DRAFT.md`

## Deliverables
1. Instrumentation implementation and event schema.
2. Scenario-focused seam test lane and captured logs.
3. FEC/F3E seam analysis note with concrete API gaps.
4. One validated API improvement with tests (pending follow-up step).

## Exit Criteria
- We can explain dependency/capability/token behavior from trace evidence.
- We have a prioritized, explicit API gap list.
- At least one incremental API improvement is implemented and test-backed.

Current status (2026-03-07):
- first two criteria: satisfied
- third criterion: pending next increment
