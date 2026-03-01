# Prompt Input

## Primary Instruction

You are executing a blocker-closure run for:
- runtime: `ocaml`
- implementation id: `coreengine-ocaml-01`
- target codebase root: `engines/ocaml/coreengine-ocaml-01`
- pinned spec pack: `docs/full-engine-spec/2026-02-27`

Normative docs:
- `SPEC_v0.md`
- `ENGINE_REQUIREMENTS.md`
- `ENGINE_API.md`
- `ENGINE_CONFORMANCE_TESTS.md`

Previous blockers (must be reduced in this run):
- `runs/engine-impl/20260301-043544Z_ocaml_coreengine-ocaml-01_2026-02-27/handoff/HANDOFF.md`
- `runs/engine-impl/20260301-043544Z_ocaml_coreengine-ocaml-01_2026-02-27/validation/CONFORMANCE_MATRIX.md`

Objective:
- Close remaining high-severity blockers for practical full-v0 conformance, prioritizing:
  1) REQ-CALC-008 required function-surface implementation,
  2) deeper cycle handling for iterative/non-iterative modes,
  3) structural rewrite depth beyond text-token heuristics.

Required outputs:
- New executable conformance/function tests that prove each closed gap.
- Updated conformance matrix with expanded CT/REQ coverage.
- Truthful final status (`completed` only if full-v0 claim is substantiated).

## Follow-up Instructions

1. Start with a REQ-mapped closure plan in `execution/SESSION_LOG.md`.
2. Implement in slices and test after each slice.
3. Keep all run artifacts updated continuously.
4. Finalize with exact remaining gaps if still blocked.
