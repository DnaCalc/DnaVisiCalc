# coreengine-net-01

C#/.NET 10 compatible core engine with a thin native-export adapter.

## Architecture

- `src/Dvc.Core`: deterministic in-memory engine domain logic.
- `src/Dvc.Native`: thin export boundary (`[UnmanagedCallersOnly(EntryPoint = "dvc_*")]`) and opaque-handle marshaling.
- `tests/*`: core unit tests, interop contract tests, and end-to-end export-boundary scenario.

## Export variants

`Dvc.Native.csproj` exposes explicit build variants via `DvcExportVariant`:

- `managed-jit`: DNNE-based native export shim over managed/JIT runtime (`<EnableDynamicLoading>true</EnableDynamicLoading>` + `DNNE` package).
- `native-aot` (default): NativeAOT shared library publish (`<PublishAot>true</PublishAot>`, `<NativeLib>Shared</NativeLib>`).

All C ABI methods remain defined once via `[UnmanagedCallersOnly(EntryPoint = "dvc_*")]`.

## Build commands

- Managed/JIT (DNNE export):
  - `dotnet build engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -p:DvcExportVariant=managed-jit -v minimal`
  - native export artifact: `src/Dvc.Native/bin/Release/net10.0/win-x64/Dvc.NativeNE.dll`.
  - keep sidecars (`Dvc.Native.dll`, `.runtimeconfig.json`, `.deps.json`) next to the DNNE export DLL for runtime bootstrap.
- NativeAOT:
  - `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -p:DvcExportVariant=native-aot -v minimal`
  - publish artifact: `src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`.
  - if linker tool resolution is not preconfigured in your shell, set `LIB` (MSVC + Windows SDK lib dirs) and pass explicit `CppLinker`/`CppLibCreator` MSVC paths.

## Test quick command

- `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
