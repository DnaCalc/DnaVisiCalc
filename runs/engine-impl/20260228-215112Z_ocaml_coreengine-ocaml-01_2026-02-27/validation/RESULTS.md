# Validation Results

Record command outputs or concise summaries.

- command: `ocamlc -version`
  status: pass
  notes: `5.2.1`

- command: `dune --version`
  status: pass
  notes: `3.21.1`

- command: `x86_64-w64-mingw32-gcc --version`
  status: pass
  notes: reports GCC `13.4.0`.

- command: `dune runtest`
  status: pass
  notes: local OCaml test target (`tests/ocaml_core_test.ml`) succeeded.

- command: `x86_64-w64-mingw32-gcc -shared ... -o dist/dvc_coreengine_ocaml01.dll src/dvc_engine.c`
  status: pass
  notes: DLL build succeeded; warning emitted: `change_push_diag` defined but not used.

- command: `objdump export count`
  status: pass
  notes: `EXPORTED_DVC_SYMBOLS=104` (matches 104 API signatures extracted from `ENGINE_API.md`).

- command: `x86_64-w64-mingw32-gcc -O2 -std=c11 -I src -o dist/api_smoke.exe tests/api_smoke.c`
  status: pass
  notes: smoke executable compiled successfully.

- command: `api_smoke.exe`
  status: pass
  notes: `EXITCODE:0`.

## Evidence interpretation

- Build/export evidence is present for a Windows-loadable DLL with the full required `dvc_*` symbol surface.
- Validation demonstrates smoke-level API functionality only.
- Full spec/API conformance remains unproven and is blocked by known implementation gaps listed in `validation/SUMMARY.yaml` and `handoff/HANDOFF.md`.
