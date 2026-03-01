# Run-Specific Additional Requirements

Run type:
- Performance-oriented refactor/hardening run for existing implementation (`coreengine-net-01`).

Scope:
- Prioritize recalc throughput/scaling improvements while preserving spec behavior.
- Keep all edits confined to:
  - `engines/dotnet/coreengine-net-01/**`
  - `runs/engine-impl/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/**`

Validation requirements:
1. Build + native artifact:
   - produce usable .NET native export DLL at:
     - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`
2. Correctness:
   - run .NET implementation tests in `engines/dotnet/coreengine-net-01/tests/*`.
   - run backend-pinned conformance smoke:
     - `DNAVISICALC_COREENGINE=dotnet-core`
     - `DNAVISICALC_COREENGINE_DLL=<dotnet dll path>`
     - `cargo test -p dnavisicalc-engine --test conformance_smoke`
3. Performance:
   - run `engine_perf_compare` with backend-pinned DLL paths and capture results.
   - include at least:
     - tiny profile (`--formula-cols 3 --formula-rows 6 --full-data false`)
     - moderate profile that still completes in practical time for .NET.
   - compare against baseline in `inputs/ISSUE_ARTIFACTS.md`.

Non-goals:
- No spec-surface expansion unless required to keep conformance behavior.
- No rewrite of repository-wide harnesses as a prerequisite for engine speedup.

Risk focus:
- semantic drift during dependency/invalidation refactor,
- epoch/stale semantics regressions,
- structural rewrite or spill behavior regressions caused by caching shortcuts.
