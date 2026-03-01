# Run-Specific Additional Requirements

## Run Type

- Iterative conformance-closure run (not first implementation bootstrap).
- Previous run status was `blocked`; this run must close a measurable subset of those blockers.

## Implementation Constraints

- Keep ownership scope limited to:
  - `engines/ocaml/coreengine-ocaml-01/**`
  - `runs/engine-impl/20260228-222210Z_ocaml_coreengine-ocaml-01_2026-02-27/**`
- Preserve all existing exported `dvc_*` symbols and ABI shape already produced.
- Maintain deterministic behavior for identical API call sequences.

## Conformance Focus

- Prioritize behavior with highest contract risk:
  - dynamic arrays (`SEQUENCE`, `RANDARRAY`, spill semantics),
  - lambda/map semantics (`LET`, `LAMBDA`, `MAP`),
  - structural rewrite correctness and rejection semantics,
  - cycle diagnostics and non-iterative behavior,
  - recalc/invalidation pathways (`invalidate_volatile`, `invalidate_udf`, `tick_streams`, explicit recalc).

## Evidence Requirements

- Include at least one non-smoke test per focused area above.
- Record command output and pass/fail result for each test run.
- In `handoff/HANDOFF.md`, explicitly separate:
  - gaps closed in this run,
  - remaining open gaps,
  - recommended next closure slice.
