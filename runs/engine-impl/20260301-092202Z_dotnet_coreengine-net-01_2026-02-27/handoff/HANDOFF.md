# Handoff

## Outcome

- status: completed
- implementation: coreengine-net-01
- spec_pack: 2026-02-27

## What Was Implemented

- Added explicit dual export configuration in `src/Dvc.Native/Dvc.Native.csproj` via `DvcExportVariant`:
  - `managed-jit` (DNNE-based managed/JIT export path),
  - `native-aot` (existing NativeAOT shared library publish path; remains default).
- Added DNNE C99 ABI type declarations and parameter-level C99 type mapping annotations for exported `dvc_*` signatures so managed/JIT export generation compiles for the existing struct/enum-heavy C ABI.
- Kept the NativeAOT entrypoint/export implementation intact (same `UnmanagedCallersOnly(EntryPoint = "dvc_*")` methods and same publish artifact location).
- Updated engine README with explicit command/configuration documentation for both variants and managed-runtime sidecar requirements.

## Validation Highlights

- Managed/JIT export artifact (loadable):
  - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/Dvc.NativeNE.dll`
- NativeAOT export artifact (loadable):
  - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`
- Export parity:
  - `dumpbin /exports` reports `104` `dvc_*` exports for both artifacts.
- Backend-pinned conformance smoke:
  - managed/JIT DNNE DLL: `pass` (`13/13`),
  - native-aot DLL: `pass` (`13/13`).

## Interop / Environment Caveat

- In this workstation environment, NativeAOT publish required explicit linker environment for reliable success:
  - `LIB` path set to MSVC + Windows SDK lib directories,
  - explicit `CppLinker`/`CppLibCreator` MSVC tool paths,
  - `IlcUseEnvironmentalTools=true`.
- Without this, publish could resolve an incompatible `link` tool and fail (`LNK1181` / `link` operand parsing errors).

## Next Suggested Run

- Optional cleanup run: centralize the NativeAOT linker-environment requirements into a checked-in helper script/target (for reproducible one-command local/CI invocation).
