# Validation Commands

List exact commands executed for this run.

## Environment

- OS: Microsoft Windows NT 10.0.26200.0
- Runtime/toolchain versions:
  - dune 3.21.1
  - OCaml 5.2.1
  - x86_64-w64-mingw32-gcc (GCC) 13.4.0

## Commands

1. `dune runtest` (workdir: `engines/ocaml/coreengine-ocaml-01`)
2. `cmd /c "if not exist dist mkdir dist && x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -DDVC_EXPORTS -I src -shared -o dist\dvc_coreengine_ocaml01.dll src\dvc_engine.c -Wl,--out-implib,dist\libdvc_coreengine_ocaml01.a && x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -I src -o dist\api_smoke.exe tests\api_smoke.c -Ldist -ldvc_coreengine_ocaml01 && x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -I src -o dist\api_closure.exe tests\api_closure.c -Ldist -ldvc_coreengine_ocaml01 && x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -I src -o dist\api_conformance_ct.exe tests\api_conformance_ct.c -Ldist -ldvc_coreengine_ocaml01"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
3. `cmd /c "cd dist && api_smoke.exe && api_closure.exe && api_conformance_ct.exe"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
4. `cmd /c "x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -DDVC_EXPORTS -I src -shared -o dist\dvc_coreengine_ocaml01.dll src\dvc_engine.c -Wl,--out-implib,dist\libdvc_coreengine_ocaml01.a && x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -I src -o dist\api_closure.exe tests\api_closure.c -Ldist -ldvc_coreengine_ocaml01 && cd dist && api_closure.exe"` (slice-level closure checks; workdir: `engines/ocaml/coreengine-ocaml-01`)
5. `$count = (x86_64-w64-mingw32-nm -g dist/dvc_coreengine_ocaml01.dll | rg " T dvc_" | Measure-Object -Line).Lines; Write-Output "EXPORTED_DVC_SYMBOLS=$count"` (workdir: `engines/ocaml/coreengine-ocaml-01`)
