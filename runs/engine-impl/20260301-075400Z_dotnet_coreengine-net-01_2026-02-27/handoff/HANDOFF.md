# Handoff

## Outcome

- status: completed
- implementation: coreengine-net-01
- spec_pack: 2026-02-27

## What Was Implemented

- Recalc pipeline refactor in `Dvc.Core` to eliminate repeated recursive subgraph reevaluation:
  - added per-recalc memoization caches for cell and name evaluation,
  - reused visiting sets in recalc loop to reduce allocation churn,
  - kept cycle fallback semantics and epoch behavior intact.
- Formula parse-path refactor:
  - parser no longer embeds runtime resolver delegates in AST nodes,
  - formula inputs now persist parsed AST (`InputEntry.ParsedFormula`) at mutation time,
  - evaluator consumes cached AST and falls back to parse only when needed.
- Structural rewrite path now stores parsed rewritten formulas when parseable and tolerates `#REF!`-containing rewrites via null parsed cache.
- Added perf-sensitive regression test:
  - `Recalculate_EvaluatesSharedDependencyOncePerPass` in `tests/Dvc.Core.Tests/CoreEngineTests.cs`.

## Validation Highlights

- Native artifact published:
  - `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`
- .NET tests:
  - `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal` -> pass (`27/27`).
- Backend-pinned conformance smoke:
  - `DNAVISICALC_COREENGINE=dotnet-core`
  - `DNAVISICALC_COREENGINE_DLL=<published dll>`
  - `cargo test -p dnavisicalc-engine --test conformance_smoke` -> pass (`13/13`).
- Perf compare (exact outputs in `.tmp/runs/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/perf/`):
  - tiny mean recalc: `48.96 ms` -> `0.04 ms` (`1224.00x`)
  - moderate mean recalc: `10244.81 ms` -> `0.16 ms` (`64030.06x`)

## Open Items / Residual Risk

- Dependency invalidation/dirty-closure scheduler is still not explicit graph-incremental; this run removes redundant reevaluation and parse overhead but still top-level traverses all formula inputs each recalc.
- Runtime remains sensitive to NativeAOT linker environment (MSVC `PATH`/`LIB` setup required in this workstation context).
- Formula-surface parity gaps from earlier staged runs remain out-of-scope for this performance-focused refactor.

## Next Suggested Run

- Implement explicit dependency graph dirty-closure scheduling to avoid top-level full-formula traversal on localized edits while preserving current spec-visible semantics.
