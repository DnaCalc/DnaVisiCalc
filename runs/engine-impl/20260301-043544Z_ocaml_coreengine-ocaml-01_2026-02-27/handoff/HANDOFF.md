# Handoff

## Outcome

- status: blocked
- implementation: coreengine-ocaml-01
- spec_pack: 2026-02-27

## What Was Implemented

- Added CT-ID mapped conformance test binary:
  - `engines/ocaml/coreengine-ocaml-01/tests/api_conformance_ct.c`
  - Covers `CT-EPOCH-001`, `CT-EPOCH-002`, `CT-CELL-001`, `CT-DET-001`, `CT-STR-001`, `CT-CYCLE-001`.
- Rebuilt DLL with MinGW toolchain and validated:
  - `dist/dvc_coreengine_ocaml01.dll`
  - `dist/api_smoke.exe`
  - `dist/api_closure.exe`
  - `dist/api_conformance_ct.exe`
- Mandatory CT evidence run passed for all six CT IDs.
- Regression reruns passed (`dune runtest`, `api_smoke`, `api_closure`).
- DLL export surface preserved:
  - `EXPORTED_DVC_SYMBOLS=104`.
- Run artifacts updated:
  - `validation/CONFORMANCE_MATRIX.md`
  - `validation/COMMANDS.md`
  - `validation/RESULTS.md`
  - `validation/SUMMARY.yaml`
  - `execution/SESSION_LOG.md`
  - `execution/TOOL_LOG.jsonl`
  - `RUN_MANIFEST.yaml`
  - `outputs/CODEBASE_REF.yaml`
  - `outputs/OUTPUT_HASHES.json`

## Open Items

- REQ-CALC-008 function-surface parity remains incomplete. Missing runtime semantics include:
  - `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
  - `IF`, `IFERROR`, `IFNA`, `NA`, `ERROR`
  - `AND`, `OR`, `NOT`, `ISERROR`, `ISNA`, `ISBLANK`, `ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`
  - `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`
  - `CONCAT`, `LEN`, `ROW`, `COLUMN`
- REQ-CALC-004 is partial:
  - non-iterative cycle fallback + diagnostic path is validated,
  - general SCC iterative convergence semantics are not implemented.
- REQ-STR-002/008/009 depth remains partial:
  - structural rewrite coverage is still text-token based and needs full grammar-case validation.
- Full v0 conformance claim therefore remains blocked despite mandatory CT-ID evidence pass.

## Next Suggested Run

- `2026-03-xx_ocaml_coreengine-ocaml-01_full-v0-gap-closure-pass`
  1. Implement the remaining REQ-CALC-008 function set with deterministic semantics.
  2. Upgrade cycle handling from simple loop detection to SCC-based iterative/non-iterative mode behavior.
  3. Add structural rewrite conformance corpus for mixed/absolute refs, ranges, spill references, and invalidation paths across complex formulas/names.
  4. Extend CT/REQ matrix with new executable cases and keep DLL export count unchanged.
