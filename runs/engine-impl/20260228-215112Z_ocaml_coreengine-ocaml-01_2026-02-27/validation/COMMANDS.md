# Validation Commands

List exact commands executed for this run.

## Environment

- OS: Microsoft Windows NT 10.0.26200.0
- Shell: PowerShell 7.5.4
- Runtime/toolchain versions:
  - ocamlc 5.2.1
  - dune 3.21.1
  - x86_64-w64-mingw32-gcc 13.4.0

## Commands

1. `ocamlc -version`
2. `dune --version`
3. `x86_64-w64-mingw32-gcc --version`
4. `dune runtest`
5. `x86_64-w64-mingw32-gcc -shared -O2 -std=c11 -Wall -Wextra -DDVC_EXPORTS -I src -o dist/dvc_coreengine_ocaml01.dll src/dvc_engine.c`
6. `$raw = x86_64-w64-mingw32-objdump -p dist/dvc_coreengine_ocaml01.dll; $count = ($raw | Select-String -Pattern '^\s*\[\s*\d+\].*dvc_').Count; Write-Output "EXPORTED_DVC_SYMBOLS=$count"`
7. `x86_64-w64-mingw32-gcc -O2 -std=c11 -I src -o dist/api_smoke.exe tests/api_smoke.c`
8. `cmd /c "cd /d dist && api_smoke.exe & echo EXITCODE:%errorlevel%"`
