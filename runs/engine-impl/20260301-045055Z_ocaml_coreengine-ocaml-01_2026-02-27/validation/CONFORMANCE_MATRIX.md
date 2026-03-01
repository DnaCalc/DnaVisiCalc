# Conformance Matrix (CT-ID + REQ Focus)

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

## Run-Targeted REQ Closure Evidence

| REQ / Area | Prior Status | Current Status | Evidence |
|---|---|---|---|
| REQ-CALC-008 (function surface) | blocked (major gaps) | pass (run-targeted) | `src/dvc_engine.c` (`try_eval_required_fn` + fallback integration), `tests/api_closure.c` (`test_slice_a_function_semantics`) covering required groups |
| REQ-CALC-004 (cycle mode depth) | partial | pass (run-targeted depth uplift) | generalized dependency-based cycle detection + iterative refinement loop in `src/dvc_engine.c`; tests `test_slice_c_cycle_diagnostic` and `test_slice_d_cycle_mode_depth` |
| REQ-STR-002/008/009 (rewrite-depth evidence) | partial | pass (run-targeted evidence uplift) | expanded rewrite assertions in `tests/api_closure.c` (`test_slice_b_rewrite_and_reject`) for absolute/mixed refs, ranges, spill refs, name rewrite, row+col edits; anchor-preservation fix in `rewrite_ref_token` |

## Export Surface Check

- `EXPORTED_DVC_SYMBOLS=104` via `x86_64-w64-mingw32-nm` on `dist/dvc_coreengine_ocaml01.dll`.

## Overall Run Verdict

- Mandatory CT set: **pass**.
- Prior high-severity blocker set in this run scope: **closed**.
- Run status: **completed**.
