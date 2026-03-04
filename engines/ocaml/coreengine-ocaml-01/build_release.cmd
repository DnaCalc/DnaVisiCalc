@echo off
setlocal enabledelayedexpansion

cd /d "%~dp0"
if not exist dist mkdir dist
if not exist tmp mkdir tmp

rem Critical on this toolchain: avoids longcmd path mangling in ocamlopt link wrappers.
set TEMP=tmp/
set TMP=tmp/

echo [1/5] dune native build
call dune build || goto :fail

echo [2/5] build OCaml+C ABI DLL directly
call ocamlfind ocamlopt -thread -package incremental,core -linkpkg -linkall ^
  -I _build/default/src ^
  -ccopt -Isrc ^
  -cclib -L/usr/x86_64-w64-mingw32/sys-root/mingw/lib ^
  -cclib -L/usr/x86_64-w64-mingw32/lib ^
  -output-complete-obj ^
  -o dist/dvc_coreengine_ocaml01.dll ^
  _build/default/src/coreengine_ocaml01.cmxa src/dvc_engine.c || goto :fail

echo [3/5] build C ABI compliance executables
call x86_64-w64-mingw32-gcc -O2 -std=c11 -I src -o dist/api_smoke.exe tests/api_smoke.c dist/dvc_coreengine_ocaml01.dll || goto :fail
call x86_64-w64-mingw32-gcc -O2 -std=c11 -I src -o dist/api_closure.exe tests/api_closure.c dist/dvc_coreengine_ocaml01.dll || goto :fail
call x86_64-w64-mingw32-gcc -O2 -std=c11 -I src -o dist/api_conformance_ct.exe tests/api_conformance_ct.c dist/dvc_coreengine_ocaml01.dll || goto :fail

echo [4/5] run C ABI compliance executables
pushd dist
call api_smoke.exe
if not "%errorlevel%"=="0" (
  echo api_smoke.exe failed with exit code %errorlevel%
  goto :fail_popd
)
call api_closure.exe
if not "%errorlevel%"=="0" (
  echo api_closure.exe failed with exit code %errorlevel%
  goto :fail_popd
)
call api_conformance_ct.exe
if not "%errorlevel%"=="0" (
  echo api_conformance_ct.exe failed with exit code %errorlevel%
  goto :fail_popd
)
popd

echo [5/5] done
echo BUILD_OK
exit /b 0

:fail_popd
popd

:fail
echo BUILD_FAILED
exit /b 1

