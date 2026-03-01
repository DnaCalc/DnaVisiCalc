@echo off
setlocal

set CL_EXE=C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe
set MSVC=C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207
set SDK=C:\Program Files (x86)\Windows Kits\10
set SDKVER=10.0.26100.0
set INC_ARGS=/I"%MSVC%\include" /I"%SDK%\Include\%SDKVER%\ucrt" /I"%SDK%\Include\%SDKVER%\shared" /I"%SDK%\Include\%SDKVER%\um" /I"%SDK%\Include\%SDKVER%\winrt" /I"C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01\src"
set LIB_ARGS=/LIBPATH:"%MSVC%\lib\x64" /LIBPATH:"%SDK%\Lib\%SDKVER%\ucrt\x64" /LIBPATH:"%SDK%\Lib\%SDKVER%\um\x64"

cd /d C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01
if errorlevel 1 (
  echo ERROR: failed to cd to engine directory
  exit /b 1
)

"%CL_EXE%" /nologo /std:c11 /W4 /wd4273 /LD /DDVC_EXPORTS %INC_ARGS% src\dvc_engine.c /Fe:dist\dvc_coreengine_ocaml01.dll /link %LIB_ARGS%
if errorlevel 1 exit /b 1

"%CL_EXE%" /nologo /std:c11 /W4 %INC_ARGS% tests\api_smoke.c /Fe:dist\api_smoke.exe /link %LIB_ARGS%
if errorlevel 1 exit /b 1

"%CL_EXE%" /nologo /std:c11 /W4 /wd4273 /wd4217 /DDVC_EXPORTS %INC_ARGS% tests\api_closure.c src\dvc_engine.c /Fe:dist\api_closure.exe /link %LIB_ARGS%
if errorlevel 1 exit /b 1

dist\api_smoke.exe
if errorlevel 1 exit /b 1

dist\api_closure.exe
if errorlevel 1 exit /b 1

echo run_closure_checks: ok
exit /b 0
