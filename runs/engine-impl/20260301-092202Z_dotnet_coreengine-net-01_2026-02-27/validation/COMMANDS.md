# Validation Commands

List exact commands executed for this run.

## Environment

- OS: Microsoft Windows 11 Pro 10.0.26200
- Runtime/toolchain versions:
  - dotnet 10.0.200-preview.0.26103.119
  - cargo 1.93.1 (083ac5135 2025-12-15)
  - rustc 1.93.1 (01f6ddf75 2026-02-11)

## Commands

1. NativeAOT publish build (explicit MSVC linker path + LIB in this environment)

```powershell
$env:NUGET_PACKAGES = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\nuget-packages"
$env:DOTNET_CLI_HOME = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\dotnet-home"
$env:LIB = "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Tools\MSVC\14.50.35717\lib\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64"
dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -p:DvcExportVariant=native-aot -p:IlcUseEnvironmentalTools=true -p:CppLinker="C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Tools\MSVC\14.50.35717\bin\Hostx64\x64\link.exe" -p:CppLibCreator="C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Tools\MSVC\14.50.35717\bin\Hostx64\x64\lib.exe" -v minimal
```

2. Managed/JIT DNNE build (run after native publish to restore managed runtime sidecars)

```powershell
$env:NUGET_PACKAGES = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\nuget-packages"
$env:DOTNET_CLI_HOME = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\dotnet-home"
dotnet build engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -p:DvcExportVariant=managed-jit -v minimal
```

3. Export parity check

```powershell
& "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Tools\MSVC\14.50.35717\bin\Hostx64\x64\dumpbin.exe" /exports engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/Dvc.NativeNE.dll
& "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Tools\MSVC\14.50.35717\bin\Hostx64\x64\dumpbin.exe" /exports engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll
```

4. Conformance smoke against managed/JIT DNNE export

```powershell
$env:CARGO_HOME = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\cargo-home"
$env:CARGO_TARGET_DIR = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\cargo-target-managed"
$env:DNAVISICALC_COREENGINE = "dotnet-core"
$env:DNAVISICALC_COREENGINE_DLL = "C:\Work\DnaCalc\DnaVisiCalc\engines\dotnet\coreengine-net-01\src\Dvc.Native\bin\Release\net10.0\win-x64\Dvc.NativeNE.dll"
cargo test -p dnavisicalc-engine --test conformance_smoke
```

5. Conformance smoke against NativeAOT export

```powershell
$env:CARGO_HOME = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\cargo-home"
$env:CARGO_TARGET_DIR = "C:\Work\DnaCalc\DnaVisiCalc\.tmp\runs\20260301-092202Z_dotnet_coreengine-net-01_2026-02-27\cargo-target-nativeaot"
$env:DNAVISICALC_COREENGINE = "dotnet-core"
$env:DNAVISICALC_COREENGINE_DLL = "C:\Work\DnaCalc\DnaVisiCalc\engines\dotnet\coreengine-net-01\src\Dvc.Native\bin\Release\net10.0\win-x64\publish\Dvc.Native.dll"
cargo test -p dnavisicalc-engine --test conformance_smoke
```
