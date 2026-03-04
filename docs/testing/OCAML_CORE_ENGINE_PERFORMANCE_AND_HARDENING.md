# OCaml Core Engine - Performance and Hardening

Date: 2026-03-04

## Scope

This report summarizes the current OCaml core-engine performance/hardening state, with emphasis on:

- whether Jane Street `Incremental` is being used on its happy path,
- what the hot path actually is,
- whether the current cross-engine benchmark is a good fit for incremental recompute.

The OCaml engine discussed here is the pure OCaml implementation behind the C ABI bridge (`dvc_engine.c` as glue only).

## Hardening Changes Landed

Recent OCaml engine/runtime changes (already in tree) that materially improved reliability:

1. Incremental height hardening
- File: `engines/ocaml/coreengine-ocaml-01/src/incremental_runtime.ml`
- Added a height budget guard (`set_max_height_allowed` to 8192).
- Replaced deep chained `map2` dependency-signature construction with `reduce_balanced`.
- Result: eliminated observed crash pattern (`Height 129 > Max 128`) on larger sheets.

2. Incremental failure containment
- File: `engines/ocaml/coreengine-ocaml-01/src/engine.ml`
- Wrapped Incremental rebuild/stabilize in `try/with`.
- On failure: disable incremental path, mark dependency layout dirty, emit diagnostic, continue via full recompute.
- Result: no process abort when Incremental throws.

3. Recalc gating/caching cleanup
- File: `engines/ocaml/coreengine-ocaml-01/src/engine.ml`
- Cached dynamic-array/volatile feature flags instead of rescanning every recalc.
- Result: reduced avoidable per-recalc overhead.

## Baseline Scaling Signal (5-profile run)

Source run:
- `.tmp/perf_scaling_5profiles_postfix_20260304T062035Z`

Key OCaml means (complex formula workload used by `engine_perf_compare`):

| Job | Formula region | Iterations | OCaml mean recalc |
|---|---:|---:|---:|
| j01 | 12x48 | 12 | 8.37 ms |
| j02 | 20x90 | 16 | 22.91 ms |
| j03 | 28x130 | 24 | 51.23 ms |
| j04 | 36x170 | 32 | 87.84 ms |
| j05 | 44x210 | 40 | 141.59 ms |
| j06 | 52x230 | 60 | 187.21 ms |

Run halt reason:
- `wall time >10s at j06/ocaml-core: 11.882s`

## Why Incremental Looks Underwhelming In This Benchmark

The benchmark mutates two inputs per iteration:
- top-row input `(col, 1)`,
- left-column input `(1, row)`,
while each formula uses `up`, `left`, and `diag`.

This creates a very broad dependency fan-out. For the benchmark mutation sequence (`col=(i*17)%C+1`, `row=(i*29)%R+1`), expected changed-formula coverage is high:

| Job | Total formulas | Avg changed | Avg ratio |
|---|---:|---:|---:|
| j01 | 517 | 424.8 | 82.2% |
| j02 | 1691 | 1245.0 | 73.6% |
| j03 | 3483 | 2725.5 | 78.3% |
| j04 | 5915 | 4592.4 | 77.6% |
| j05 | 8987 | 6927.2 | 77.1% |
| j06 | 11679 | 9041.8 | 77.4% |

So Incremental is often asked to recompute most formulas anyway.

## Targeted Probes (j06 shape, OCaml + Rust)

Probe run dir:
- `.tmp/ocaml_hotpath_probe_20260304T082551Z`

I extended `engine_perf_compare` with benchmark-only knobs:
- `--fixed-mutation-col/--fixed-mutation-row`
- `--force-iteration-enabled` (used to disable OCaml incremental path for A/B)
- `--simple-formula` (replace complex formula with `=up+left+diag`)

### Complex formula (default harness expression)

| Engine | Mutation mode | Incremental mode | Mean recalc |
|---|---|---|---:|
| OCaml | sweep(col,row) | ON | 194.00 ms |
| OCaml | sweep(col,row) | OFF (`--force-iteration-enabled`) | 226.13 ms |
| OCaml | fixed(52,230) | ON | 6.15 ms |
| OCaml | fixed(52,230) | OFF (`--force-iteration-enabled`) | 230.11 ms |
| Rust | sweep(col,row) | ON | 20.28 ms |
| Rust | fixed(52,230) | ON | 1.46 ms |

### Simple formula (`=up+left+diag`)

| Engine | Mutation mode | Incremental mode | Mean recalc |
|---|---|---|---:|
| OCaml | sweep(col,row) | ON | 26.37 ms |
| OCaml | sweep(col,row) | OFF (`--force-iteration-enabled`) | 18.79 ms |
| OCaml | fixed(52,230) | ON | 0.95 ms |
| OCaml | fixed(52,230) | OFF (`--force-iteration-enabled`) | 18.68 ms |
| Rust | sweep(col,row) | ON | 6.45 ms |
| Rust | fixed(52,230) | ON | 1.05 ms |

## Optimization Pass Implemented (2026-03-04)

Implemented in:
- `engines/ocaml/coreengine-ocaml-01/src/eval.ml`
- `engines/ocaml/coreengine-ocaml-01/src/engine.ml`

Change:
- Added a compiled-expression fast path for arithmetic/cell-ref/comparison formulas plus `ROUND(...)`, with strict fallback to the existing text evaluator for unsupported constructs.
- Added per-cell compiled formula cache keyed by formula text.
- Kept semantics-safe fallback (`Eval.eval_expr`) for full spec surface.

Why this target:
- Earlier probes identified repeated string parse/eval as the dominant OCaml hot path for complex arithmetic formulas.

## Before/After OCaml Results (profile matrix)

Runs:
- before: `.tmp/engine_profile_matrix_before_ocaml_opt_20260304T140131Z`
- after: `.tmp/engine_profile_matrix_after_ocaml_opt_20260304T140559Z`

| Profile | Workload shape | OCaml before | OCaml after | Delta |
|---|---|---:|---:|---:|
| p01_broad_complex_mid | broad, complex, 20x90 | 23.68 ms | 2.44 ms | -89.70% |
| p02_broad_complex_large | broad, complex, 52x230 | 175.34 ms | 20.79 ms | -88.14% |
| p03_localized_complex_large | localized, complex, 52x230 | 5.69 ms | 0.62 ms | -89.10% |
| p04_localized_simple_large | localized, simple, 52x230 | 0.68 ms | 0.57 ms | -16.18% |
| p05_broad_simple_large | broad, simple, 52x230 | 23.24 ms | 14.30 ms | -38.47% |
| p06_row_sweep_complex_large | mixed locality, complex, 52x230 | 118.33 ms | 12.00 ms | -89.86% |

Observed outcome:
- OCaml moved from a clear outlier to competitive or leading on several profiles.
- Biggest gains are exactly where formula parse/eval dominated.

## Conclusions

1. Incremental is now functioning and useful
- On localized updates, OCaml incremental gives a major win (e.g. `230.11 -> 6.15 ms`, ~37x).
- This confirms the Incremental path is materially active, not placebo.

2. Current cross-engine benchmark is mostly a broad-fanout stress test
- With ~77% changed formulas per recalc on average, Incremental pruning is naturally limited.
- So poor OCaml numbers on this benchmark do not imply Incremental is broken.

3. OCaml hot path was formula evaluation throughput on complex expressions
- Simple formulas are much faster (`194.00 -> 26.37 ms` under sweep, same shape).
- The dominant cost was OCaml text-formula parse/eval work (string slicing/scanning/recursive expression handling), not Incremental scheduling alone.
- The compiled-expression fast path materially reduced this bottleneck in practice.

4. Incremental has a crossover point
- For cheap formulas + broad invalidation, incremental bookkeeping can cost more than full recompute (`26.37 ms` vs `18.79 ms`).
- For expensive formulas or localized invalidation, incremental wins decisively.

## Updated Next Steps

1. Extend compiler coverage
- Add support for more built-ins and named references in compiled mode.
- Keep fallback-on-miss behavior to preserve compatibility.

2. Introduce adaptive recompute policy
- Keep Incremental for sparse invalidation.
- Consider full-path fallback when predicted changed region is very high and formula-cost signature is low.
- Requires a light heuristic (changed-set estimate + formula complexity marker).

3. Keep two benchmark classes in performance reporting
- `broad-fanout` (current sweep) and `localized` (fixed near bottom-right).
- This prevents misreading “Incremental quality” from one mutation topology.

4. Add evaluator-focused microbench suite (OCaml-only)
- Compare complex arithmetic, function-heavy, and simple reference formulas.
- Track parser/eval throughput separately from dependency propagation.

## Compliance Check Note

- C ABI conformance executables pass in `build_release.cmd`.
- Rust conformance smoke against OCaml DLL passed with single-thread test harness:
  `cargo test -p dnavisicalc-engine --test conformance_smoke -- --test-threads=1`

## Repro Commands (examples)

From repo root:

```powershell
# OCaml complex sweep
target/release/engine_perf_compare.exe `
  --iterations 60 --full-data true --formula-cols 52 --formula-rows 230 `
  --include-ocaml --backend ocaml-core `
  --ocaml-dll engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll

# OCaml complex fixed localized mutation
target/release/engine_perf_compare.exe `
  --iterations 60 --full-data true --formula-cols 52 --formula-rows 230 `
  --include-ocaml --backend ocaml-core `
  --ocaml-dll engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll `
  --fixed-mutation-col 52 --fixed-mutation-row 230

# OCaml A/B with incremental disabled (probe mode)
target/release/engine_perf_compare.exe `
  --iterations 60 --full-data true --formula-cols 52 --formula-rows 230 `
  --include-ocaml --backend ocaml-core `
  --ocaml-dll engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll `
  --force-iteration-enabled
```
