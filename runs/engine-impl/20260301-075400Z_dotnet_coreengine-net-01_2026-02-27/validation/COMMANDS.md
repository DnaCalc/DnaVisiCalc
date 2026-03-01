# Validation Commands

## Environment

- OS: Windows (PowerShell)
- dotnet: `10.0.200-preview.0.26103.119`
- cargo: `1.93.1 (083ac5135 2025-12-15)`
- rustc: `1.93.1 (01f6ddf75 2026-02-11)`

## Commands

1. `dotnet --version`
2. `cargo --version`
3. `rustc --version`
4. `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (baseline attempt; failed due linker PATH)
5. `$msvc='C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207'; $kit='C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0'; $env:PATH="$msvc\bin\Hostx64\x64;C:\Program Files\dotnet;C:\Windows\System32;C:\Windows"; $env:LIB="$msvc\lib\x64;$kit\um\x64;$kit\ucrt\x64"; dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (baseline publish with MSVC env)
6. `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 3 --formula-cols 3 --formula-rows 6 --full-data false --backend dotnet-core --dotnet-dll engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll --output .tmp/runs/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/perf/before_tiny.txt`
7. `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 2 --formula-cols 4 --formula-rows 8 --full-data false --backend dotnet-core --dotnet-dll engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll --output .tmp/runs/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/perf/before_moderate.txt`
8. `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
9. `$msvc='C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\14.44.35207'; $kit='C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0'; $env:PATH="$msvc\bin\Hostx64\x64;C:\Program Files\dotnet;C:\Windows\System32;C:\Windows"; $env:LIB="$msvc\lib\x64;$kit\um\x64;$kit\ucrt\x64"; dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (post-refactor publish)
10. `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 3 --formula-cols 3 --formula-rows 6 --full-data false --backend dotnet-core --dotnet-dll engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll --output .tmp/runs/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/perf/after_tiny.txt`
11. `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 2 --formula-cols 4 --formula-rows 8 --full-data false --backend dotnet-core --dotnet-dll engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll --output .tmp/runs/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/perf/after_moderate.txt`
12. `$env:DNAVISICALC_COREENGINE='dotnet-core'; $env:DNAVISICALC_COREENGINE_DLL='C:\Work\DnaCalc\DnaVisiCalc\engines\dotnet\coreengine-net-01\src\Dvc.Native\bin\Release\net10.0\win-x64\publish\Dvc.Native.dll'; cargo test -p dnavisicalc-engine --test conformance_smoke`
