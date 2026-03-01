# Validation Results

Record command outputs or concise summaries.

- command: `dune runtest` (workdir: `engines/ocaml/coreengine-ocaml-01`)
  status: pass
  notes: OCaml unit test suite passed with exit code `0`.

- command: `cmd /c runs\engine-impl\20260228-222210Z_ocaml_coreengine-ocaml-01_2026-02-27\validation\run_closure_checks.cmd`
  status: pass
  notes: Built `dvc_coreengine_ocaml01.dll`, `api_smoke.exe`, and `api_closure.exe`; runtime output included `api_smoke: ok`, `api_closure: ok`, and `run_closure_checks: ok` with exit code `0`.
