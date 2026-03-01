# Validation Results

Record command outputs or concise summaries.

- command: `dune runtest`
  status: pass
  notes: OCaml test target passed.

- command: DLL + baseline C binaries build with `x86_64-w64-mingw32-gcc`
  status: pass
  notes: Built `dist/dvc_coreengine_ocaml01.dll`, `dist/api_smoke.exe`, `dist/api_closure.exe`.

- command: `dist/api_smoke.exe`
  status: pass
  notes: Output `api_smoke: ok`.

- command: `dist/api_closure.exe`
  status: pass
  notes: Output `api_closure: ok`.

- command: DLL + CT binary build + run (`api_conformance_ct`)
  status: pass
  notes: Output:
    - `CT-EPOCH-001: pass`
    - `CT-EPOCH-002: pass`
    - `CT-CELL-001: pass`
    - `CT-DET-001: pass`
    - `CT-STR-001: pass`
    - `CT-CYCLE-001: pass`
    - `api_conformance_ct: ok`

- command: regression rerun (`dune runtest`, `api_smoke`, `api_closure`)
  status: pass
  notes: All reruns green after CT test addition.

- command: export-count check via `x86_64-w64-mingw32-nm`
  status: pass
  notes: `EXPORTED_DVC_SYMBOLS=104`.
