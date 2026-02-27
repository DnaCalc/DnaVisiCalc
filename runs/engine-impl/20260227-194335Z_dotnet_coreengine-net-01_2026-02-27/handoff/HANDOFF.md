# Handoff

## Implemented in this run

- New .NET 10 solution and project layout under `engines/dotnet/coreengine-net-01`.
- Deterministic core engine (`Dvc.Core`) with:
  - lifecycle/config APIs,
  - cells/names set/get/clear and A1 parsing support,
  - recalc modes + epochs + explicit recalc,
  - formula evaluation subset (`+ - * /`, refs, names, `SUM`, `IF`, `SEQUENCE`, `RAND`, `RANDARRAY`, `NOW`, `STREAM`),
  - basic spill semantics with anchor/member/range queries,
  - structural row/column insert/delete with deterministic A1 rewrite and `#REF!` injection,
  - formatting APIs,
  - iteration config API,
  - diagnostics (`last_error*`, `last_reject*`, cell error message, palette names, parse ref),
  - controls/charts/UDF/change-tracking surfaces and iterators.
- Thin native export adapter (`Dvc.Native`) exposing explicit `dvc_*` symbols via `UnmanagedCallersOnly`.
- Tests:
  - core unit tests,
  - interop contract tests (null-pointer handling + buffer query pattern),
  - end-to-end export-boundary scenario (formulas + structural rewrite + epochs).
- Native AOT artifact published for `win-x64` and symbol table verified.

## Artifact

- `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`

## Gap List

### missing-feature

- Formula/function surface is a strict subset of `SPEC_v0` (no full logical/comparison/text/math/trig/financial/lambda/reference-helper parity).
- Spill support is limited to evaluator-produced arrays (`SEQUENCE`/`RANDARRAY`) and does not implement full spill-reference semantics (`A1#`) across all expression paths.
- Change journal does not capture full old/new payload richness for every change entry type.

### behavior-mismatch-risk

- Cell-reference evaluation treats A1 references as direct coordinates (copy-semantics relative-reference behavior is simplified).
- Structural rewrite currently performs token-level A1 rewriting and may diverge from full mixed/absolute edge-case semantics in complex formulas.
- Iterative cycle handling is basic fixed-point retry and does not implement full SCC-targeted iteration semantics.

### interop-risk

- UDF callback bridge assumes Cdecl ABI and does not provide dynamic calling-convention negotiation.
- Export-layer contract coverage is representative, not exhaustive over every exported function/parameter combination.

### performance-risk

- Recalculation path is primarily full-sheet recompute; dirty-closure incremental optimization is not implemented.
- Structural mutations rebuild affected maps directly without graph-level optimization.
