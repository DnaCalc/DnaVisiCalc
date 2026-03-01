# Validation Commands

List exact commands executed for this run.

## Environment

- OS: Microsoft Windows 11 Pro
- Runtime/toolchain versions:
  - `cargo 1.93.1 (083ac5135 2025-12-15)`
  - `rustc 1.93.1 (01f6ddf75 2026-02-11)`
  - `x86_64-w64-mingw32-gcc (GCC) 13.4.0`

## Commands

1. `powershell -Command "$env:DNAVISICALC_COREENGINE='dotnet-core'; $env:DNAVISICALC_COREENGINE_DLL='C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01\dist\dvc_coreengine_ocaml01.dll'; cargo test -p dnavisicalc-engine --test conformance_smoke"`
2. `powershell -Command "x86_64-w64-mingw32-gcc --% -std=c11 -O2 -shared -I src -o dist/dvc_coreengine_ocaml01.dll src/dvc_engine.c -Wl,--out-implib,dist/dvc_coreengine_ocaml01.lib -Wl,--output-def,dist/dvc_coreengine_ocaml01.def"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
3. `powershell -Command "$env:DNAVISICALC_COREENGINE='dotnet-core'; $env:DNAVISICALC_COREENGINE_DLL='C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01\dist\dvc_coreengine_ocaml01.dll'; cargo test -p dnavisicalc-engine --test conformance_smoke ct_entities_001_control_roundtrip_holds -- --nocapture"`
4. `powershell -Command "$env:DNAVISICALC_COREENGINE='dotnet-core'; $env:DNAVISICALC_COREENGINE_DLL='C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01\dist\dvc_coreengine_ocaml01.dll'; cargo test -p dnavisicalc-engine --test conformance_smoke"`
5. `powershell -Command "x86_64-w64-mingw32-gcc --% -std=c11 -O2 -I src -o dist/api_conformance_ct.exe tests/api_conformance_ct.c dist/dvc_coreengine_ocaml01.lib"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
6. `powershell -Command "./dist/api_conformance_ct.exe"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
7. `powershell -Command "x86_64-w64-mingw32-nm -g engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll | rg \" T dvc_\" | Measure-Object | Select-Object -ExpandProperty Count"`
