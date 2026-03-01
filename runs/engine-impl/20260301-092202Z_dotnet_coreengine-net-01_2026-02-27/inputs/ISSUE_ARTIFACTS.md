# Issue Artifacts (Managed Export Gap)

## Current gap

Managed/JIT .NET build outputs currently fail C API loading in `dnavisicalc-engine`:
- loader error: `GetProcAddress failed`
- affected DLLs:
  - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Debug/net10.0/Dvc.Native.dll`
  - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/Dvc.Native.dll`

NativeAOT published DLL works:
- `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`

## Objective for this run

Add a managed/JIT export path (DNNE-based) that exposes required `dvc_*` symbols while preserving the NativeAOT variant.
