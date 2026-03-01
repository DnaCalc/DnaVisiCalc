# Conformance Matrix (CT-ID)

Engine: `coreengine-ocaml-01`  
Spec pack: `2026-02-27`

## Mandatory CT Coverage

| CT-ID | Invariant | Primary REQ Mapping | Status | Evidence |
|---|---|---|---|---|
| CT-EPOCH-001 | INV-EPOCH-001 (`stabilized_epoch <= committed_epoch`) | REQ-EXEC-002, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_epoch_001`), command: `dist/api_conformance_ct.exe` |
| CT-EPOCH-002 | INV-EPOCH-002 (epoch monotonicity on successful API calls) | REQ-EXEC-002, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_epoch_002`), command: `dist/api_conformance_ct.exe` |
| CT-CELL-001 | INV-CELL-001 (`stale == 1` iff `value_epoch < committed_epoch`) | REQ-CALC-006, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_cell_001`), command: `dist/api_conformance_ct.exe` |
| CT-DET-001 | INV-DET-001 deterministic replay | REQ-EXEC-004, REQ-CALC-001, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_det_001`), command: `dist/api_conformance_ct.exe` |
| CT-STR-001 | INV-STR-001 rejected structural atomicity | REQ-STR-006, REQ-STR-010, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_str_001`), command: `dist/api_conformance_ct.exe` |
| CT-CYCLE-001 | INV-CYCLE-001 non-iterative circular behavior + diagnostics | REQ-CALC-004, REQ-DELTA-004, REQ-INVSET-002 | pass | `tests/api_conformance_ct.c` (`test_ct_cycle_001`), command: `dist/api_conformance_ct.exe` |

## Export Surface Check

- `EXPORTED_DVC_SYMBOLS=104` via `x86_64-w64-mingw32-nm` on `dist/dvc_coreengine_ocaml01.dll`.

## Overall Run Verdict

- CT mandatory set above: **pass**.
- Full v0 conformance claim: **blocked** (remaining REQ-level gaps below).

## Remaining REQ/CT Gaps (Blockers for `completed`)

- REQ-CALC-008 function-surface parity is incomplete. Required functions still missing runtime semantics include:
  - `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
  - `IF`, `IFERROR`, `IFNA`, `NA`, `ERROR`
  - `AND`, `OR`, `NOT`, `ISERROR`, `ISNA`, `ISBLANK`, `ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`
  - `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`
  - `CONCAT`, `LEN`, `ROW`, `COLUMN`
- REQ-CALC-004 iterative cycle semantics are partial:
  - iteration config is exposed, but recalc cycle handling currently relies on simple single-reference-loop detection instead of full SCC iterative convergence behavior.
- REQ-STR-002/008/009 coverage depth remains incomplete:
  - rewrite logic is text-token based and not yet validated against full formula grammar/path combinations.

- Additional CT gaps:
  - For this run's mandatory CT set (`CT-EPOCH-001/002`, `CT-CELL-001`, `CT-DET-001`, `CT-STR-001`, `CT-CYCLE-001`): **none remaining**.
