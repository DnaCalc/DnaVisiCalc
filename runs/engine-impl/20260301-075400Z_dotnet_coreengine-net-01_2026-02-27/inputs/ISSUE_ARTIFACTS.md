# Issue Artifacts (Performance Baseline)

## Baseline Observation

Date: 2026-03-01  
Harness: `crates/dnavisicalc-engine/src/bin/engine_perf_compare.rs`

### Tiny profile (all three engines)

Command:
- `engine_perf_compare --iterations 3 --formula-cols 3 --formula-rows 6 --full-data false --include-ocaml ...`

Observed recalc mean:
- `dotnet-core`: `48.42 ms`
- `rust-core`: `0.23 ms`
- `ocaml-core`: `0.29 ms`

Observed p95:
- `dotnet-core`: `50.50 ms`
- `rust-core`: `0.26 ms`
- `ocaml-core`: `0.29 ms`

### Slightly larger profile probe

Command:
- `engine_perf_compare --iterations 2 --formula-cols 4 --formula-rows 8 --full-data false --backend dotnet-core ...`

Observed recalc mean:
- `dotnet-core`: `13036.58 ms`

Observed initial recalc:
- `dotnet-core`: `13026.40 ms`

## Problem Statement

Current .NET engine recalc behavior degrades sharply with workload growth and is far from Rust/OCaml performance on comparable scenarios.

This run should focus on internal algorithmic refactoring for significantly better recalc scaling while preserving spec-visible behavior and conformance.
