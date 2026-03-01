# Handoff

## Outcome

- status: completed
- implementation: coreengine-ocaml-01
- spec_pack: 2026-02-27

## What Was Implemented

- Implemented remaining REQ-CALC-008 required function groups in engine evaluation path:
  - aggregates, conditional/error, logical/predicate, numeric/scientific, financial/lookup, text/reference helpers.
- Added/expanded executable closure tests (`tests/api_closure.c`) with assertions per required function group.
- Upgraded cycle handling depth:
  - generalized formula dependency cycle detection (refs + ranges across full formula text),
  - iterative cycle refinement loop using iteration config (`enabled`, `max_iterations`, `convergence_tolerance`),
  - new tests for non-iterative multi-node cycle + diagnostic and iterative convergence behavior.
- Expanded structural rewrite-depth evidence in tests:
  - mixed absolute/relative refs, ranges, spill refs, name formulas, row/column mutation paths.
- Fixed structural rewrite anchoring bug:
  - preserved absolute row/column anchors during row/col structural rewrites (`rewrite_ref_token`).
- Preserved existing conformance baseline:
  - mandatory CT suite passing,
  - DLL export surface unchanged (`EXPORTED_DVC_SYMBOLS=104`).

## Open Items

- No high-severity blocker from the previous run remains in this owned scope.

## Next Suggested Run

- `2026-03-xx_ocaml_coreengine-ocaml-01_conformance-breadth-pass`
  1. Add broader CT-ID coverage beyond the mandatory set (additional REQ/CT rows in matrix).
  2. Extend structural rewrite corpus toward more nested grammar combinations and INDIRECT/OFFSET rewrite interaction cases.
