# coreengine-net-01

C#/.NET 10 compatible core engine with a thin native-export adapter.

## Architecture

- `src/Dvc.Core`: deterministic in-memory engine domain logic.
- `src/Dvc.Native`: thin export boundary (`[UnmanagedCallersOnly(EntryPoint = "dvc_*")]`) and opaque-handle marshaling.
- `tests/*`: core unit tests, interop contract tests, and end-to-end export-boundary scenario.

## Native export mechanism

This implementation uses .NET NativeAOT shared library publishing (`<PublishAot>true</PublishAot>`, `<NativeLib>Shared</NativeLib>`).

- Each `dvc_*` symbol is an explicit `UnmanagedCallersOnly` export.
- Export methods only validate/marshal and delegate to `Dvc.Core` behavior.
- Business logic is intentionally kept out of the export layer.

## Build/Test quick commands

- `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
- `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal`
