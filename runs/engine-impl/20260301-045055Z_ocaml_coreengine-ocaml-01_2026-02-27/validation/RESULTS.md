# Validation Results

Record command outputs or concise summaries.

- command: `dune runtest`
  status: pass
  notes: OCaml test target passed after all code/test updates.

- command: DLL + C binaries build (`dvc_coreengine_ocaml01.dll`, `api_smoke.exe`, `api_closure.exe`, `api_conformance_ct.exe`)
  status: pass
  notes: Rebuilt successfully with MinGW after each slice and for final validation.

- command: `dist/api_smoke.exe`
  status: pass
  notes: Output `api_smoke: ok`.

- command: `dist/api_closure.exe`
  status: pass
  notes: Output `api_closure: ok`; includes new function-surface, cycle-depth, and rewrite-depth assertions.

- command: `dist/api_conformance_ct.exe`
  status: pass
  notes: Output:
    - `CT-EPOCH-001: pass`
    - `CT-EPOCH-002: pass`
    - `CT-CELL-001: pass`
    - `CT-DET-001: pass`
    - `CT-STR-001: pass`
    - `CT-CYCLE-001: pass`
    - `api_conformance_ct: ok`

- command: export-count check via `x86_64-w64-mingw32-nm`
  status: pass
  notes: `EXPORTED_DVC_SYMBOLS=104` (unchanged).
