# Validation Commands

## Environment

- OS: Windows 10.0.26200 (win-x64)
- .NET SDK: 10.0.200-preview.0.26103.119
- .NET runtime: 10.0.3
- Native toolchain for publish: Visual Studio 2022 VC++ (`vcvars64.bat`)

## Commands

1. `dotnet --info`
2. `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
3. `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (failed in default shell due PATH selecting Git `link.exe`)
4. `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` with PATH prefixed to MSVC `link.exe` (failed: missing VC/SDK LIB env)
5. `cmd /c .tmp\\runs\\publish_coreengine_net01.cmd` where script sets short PATH, calls `vcvars64.bat`, then runs publish (pass)
6. `dumpbin /exports engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/x64/Release/net10.0/win-x64/publish/Dvc.Native.dll` (compared against API function list)
