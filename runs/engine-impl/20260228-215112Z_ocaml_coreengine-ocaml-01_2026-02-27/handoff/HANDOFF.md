# Handoff

## Outcome

- status: blocked
- implementation: coreengine-ocaml-01
- spec_pack: 2026-02-27

## What Was Implemented

- Created OCaml implementation tree under `engines/ocaml/coreengine-ocaml-01` with dune scaffolding and minimal OCaml module/test.
- Implemented a Windows-loadable C API DLL (`dist/dvc_coreengine_ocaml01.dll`) exporting the full 104-function `dvc_*` symbol surface defined in `ENGINE_API.md`.
- Added C header `src/dvc_engine.h` aligned to API constants/types and function declarations.
- Added C engine implementation `src/dvc_engine.c` with baseline behavior across lifecycle/cell/name/recalc/format/spill/iterator/structural/entity/UDF/change-tracking utility surfaces.
- Added smoke validator `tests/api_smoke.c` and built `dist/api_smoke.exe` (passes with `EXITCODE:0`).
- Cleaned transient files (`_build`, `src/.protos.tmp`) after validation pass.

## Validation Evidence

- `dune runtest` passed.
- DLL build succeeded with one non-fatal warning (`change_push_diag` unused).
- Export check: `EXPORTED_DVC_SYMBOLS=104`.
- API smoke executable run passed (`EXITCODE:0`).

## Policy and Clean-Room Notes

- Managed-context exception recorded: early context-loading reads occurred before context policy parse due AGENTS-required load order.
- Exception has been logged in `execution/SESSION_LOG.md` and `execution/TOOL_LOG.jsonl` with `policy: exception-approved`.
- No Rust/.NET engine implementation tree was read during this run.

## Open Items / Blockers

1. REQ-CALC-008/009 full function-surface parity is not complete; many required functions are not implemented to contract semantics.
2. REQ-STR-002..010 structural rewrite semantics for formula/name/range/spill references are incomplete relative to normative rewrite rules.
3. REQ-CALC-004 and REQ-DELTA-004 circular-reference handling/diagnostic behavior is incomplete.
4. API behavior conformance is smoke-level only; no requirement-case granularity conformance evidence against `ENGINE_CONFORMANCE_TESTS.md`.
5. Runtime architecture requirement asks for OCaml engine implementation; current delivery is C-centric with minimal OCaml module only.

## Next Suggested Run

- `2026-03-xx_ocaml_coreengine-ocaml-01_conformance-closure`:
  - replace C-centric evaluator with OCaml core + C FFI boundary,
  - implement full v0 function semantics and structural rewrite rules,
  - add requirement-mapped API tests (including cycle diagnostics, spill rejects, UDF invalidation classes, controls/charts/change journal details),
  - publish conformance matrix for `REQ-*` and `dvc_*` case coverage.
