# Handoff

## Outcome

- status: completed
- implementation: coreengine-ocaml-01
- spec_pack: 2026-02-27

## What Was Implemented

- Slice A (REQ-CALC-008/009, REQ-SPILL-001/002):
  - strengthened dynamic-array argument/default handling for `SEQUENCE` and `RANDARRAY`,
  - added stricter `MAP` source range parsing,
  - retained and validated `LET`/`LAMBDA` invocation, `INDIRECT` A1/R1C1, and `OFFSET` scalar behavior through API tests.
- Slice B (REQ-STR-002..010):
  - kept deterministic formula/name rewrite behavior for single refs/ranges/spill anchors,
  - added richer structural reject context for spill-boundary rejects (`has_cell` + `has_range` payload).
- Slice C (REQ-CALC-004, REQ-DELTA-004):
  - validated non-iterative circular-reference behavior remains non-fatal and emits diagnostic journal entries.
- Added requirement-targeted C API closure suite:
  - `engines/ocaml/coreengine-ocaml-01/tests/api_closure.c`.
- Fixed spill materialization recalc ordering bug discovered during closure testing:
  - spill-member computed values are no longer overwritten by later empty-cell pass in the same recalc.
- Updated smoke test to use non-cell-like chart identifier (`CHART_ONE`) consistent with current name-validation rules.
- Added reproducible C validation script:
  - `runs/engine-impl/20260228-222210Z_ocaml_coreengine-ocaml-01_2026-02-27/validation/run_closure_checks.cmd`.

## Open Items

- Full v0 conformance remains incomplete outside this closure slice; broad function-surface parity and deeper case-by-case conformance registry coverage are still open.
- Runtime is still C-centric with minimal OCaml module usage; full OCaml-core-first execution path is not implemented in this run.
- Current cross-engine conformance report at `CT-*` granularity is not yet published.

## Next Suggested Run

- `2026-03-xx_ocaml_coreengine-ocaml-01_conformance-matrix-pass`:
  1. Expand `api_closure` into requirement/CT-ID mapped suites (`CT-EPOCH-001`, `CT-DET-001`, `CT-STR-001`, `CT-CYCLE-001`, and additional function-surface cases).
  2. Close remaining staged critical function gaps not covered by this slice.
  3. Publish an explicit pass/fail matrix against `ENGINE_CONFORMANCE_TESTS.md`.
