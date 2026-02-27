# Validation Commands

1. `dotnet --list-sdks`
2. `dotnet build engines/dotnet/coreengine-net-01/src/Dvc.Core/Dvc.Core.csproj -v minimal`
3. `dotnet build engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -v minimal`
4. `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
5. `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (initial attempt failed due linker env)
6. `$env:Path='C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;'+$env:Path; $env:LIB='C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207\lib\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64'; dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal`
7. `dumpbin /exports engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`
