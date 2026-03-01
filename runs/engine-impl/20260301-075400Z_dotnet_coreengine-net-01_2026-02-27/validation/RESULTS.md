# Validation Results

- command: `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal` (default PATH)
  status: fail
  notes: NativeAOT link phase selected non-MSVC `link` from PATH and failed (`extra operand '/DEF:...'`).

- command: `dotnet publish ...` with explicit MSVC `PATH` + `LIB`
  status: pass
  notes: Published `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`.

- command: `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 3 --formula-cols 3 --formula-rows 6 --full-data false --backend dotnet-core --dotnet-dll ... --output .../before_tiny.txt`
  status: pass
  notes: Baseline tiny recalc mean `48.96 ms`, p95 `49.50 ms`, initial recalc `45.52 ms`.

- command: `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 2 --formula-cols 4 --formula-rows 8 --full-data false --backend dotnet-core --dotnet-dll ... --output .../before_moderate.txt`
  status: pass
  notes: Baseline moderate recalc mean `10244.81 ms`, p95 `10585.75 ms`, initial recalc `9940.39 ms`.

- command: `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
  status: pass
  notes: `27` tests passed (`Dvc.Core.Tests` 17, `Dvc.E2E.Tests` 2, `Dvc.Interop.Tests` 8).

- command: `dotnet publish ...` with explicit MSVC `PATH` + `LIB` (post-refactor)
  status: pass
  notes: Re-published NativeAOT DLL after code changes.

- command: `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 3 --formula-cols 3 --formula-rows 6 --full-data false --backend dotnet-core --dotnet-dll ... --output .../after_tiny.txt`
  status: pass
  notes: Post-refactor tiny recalc mean `0.04 ms`, p95 `0.05 ms`, initial recalc `0.12 ms`.

- command: `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- --iterations 2 --formula-cols 4 --formula-rows 8 --full-data false --backend dotnet-core --dotnet-dll ... --output .../after_moderate.txt`
  status: pass
  notes: Post-refactor moderate recalc mean `0.16 ms`, p95 `0.21 ms`, initial recalc `0.27 ms`.

- command: `$env:DNAVISICALC_COREENGINE=dotnet-core; $env:DNAVISICALC_COREENGINE_DLL=<published dll>; cargo test -p dnavisicalc-engine --test conformance_smoke`
  status: pass
  notes: Backend-pinned conformance smoke passed (`13/13`).

## Performance Comparison

- tiny profile (`--formula-cols 3 --formula-rows 6 --full-data false`)
  - recalc mean: `48.96 ms` -> `0.04 ms` (`1224.00x` faster)
  - recalc p95: `49.50 ms` -> `0.05 ms` (`990.00x` faster)
  - initial recalc: `45.52 ms` -> `0.12 ms` (`379.33x` faster)

- moderate profile (`--formula-cols 4 --formula-rows 8 --full-data false`)
  - recalc mean: `10244.81 ms` -> `0.16 ms` (`64030.06x` faster)
  - recalc p95: `10585.75 ms` -> `0.21 ms` (`50408.33x` faster)
  - initial recalc: `9940.39 ms` -> `0.27 ms` (`36816.26x` faster)
