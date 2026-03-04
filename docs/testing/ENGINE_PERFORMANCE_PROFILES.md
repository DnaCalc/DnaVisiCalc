# Engine Performance Profiles

Date: 2026-03-04

## Purpose

Performance conclusions depend heavily on workload topology. This document formalizes profile classes so cross-engine comparisons are interpretable and reproducible.

Primary runner:
- `scripts/windows/run_engine_profile_matrix.ps1`

Reference optimized run (current):
- `.tmp/engine_profile_matrix_after_ocaml_opt_20260304T140559Z`

## Canonical Profiles

| Profile ID | Intent | Key knobs |
|---|---|---|
| `p01_broad_complex_mid` | Broad fan-out, complex formulas, medium sheet | `--formula-cols 20 --formula-rows 90 --full-data false` |
| `p02_broad_complex_large` | Broad fan-out, complex formulas, large sheet | `--formula-cols 52 --formula-rows 230 --full-data true` |
| `p03_localized_complex_large` | Localized incremental-friendly updates, complex formulas | `--fixed-mutation-col 52 --fixed-mutation-row 230` |
| `p04_localized_simple_large` | Localized incremental-friendly updates, cheap formulas | `--simple-formula --fixed-mutation-col 52 --fixed-mutation-row 230` |
| `p05_broad_simple_large` | Broad fan-out with cheap formulas | `--simple-formula` |
| `p06_row_sweep_complex_large` | Mixed locality (row fixed, col sweep), complex formulas | `--fixed-mutation-row 230` |

## Current Snapshot (mean recalc, lower is better)

From `.tmp/engine_profile_matrix_after_ocaml_opt_20260304T140559Z/results.csv`.

| Profile | 1st | 2nd | 3rd | 4th | 5th |
|---|---|---|---|---|---|
| `p01_broad_complex_mid` | OCaml `2.44` | Rust `4.68` | C `4.76` | .NET AOT `10.33` | .NET JIT `16.12` |
| `p02_broad_complex_large` | Rust `19.66` | OCaml `20.79` | C `32.25` | .NET JIT `69.41` | .NET AOT `76.47` |
| `p03_localized_complex_large` | OCaml `0.62` | Rust `1.36` | C `31.64` | .NET JIT `64.39` | .NET AOT `73.87` |
| `p04_localized_simple_large` | Rust `0.57` | OCaml `0.57` | C `13.69` | .NET AOT `21.93` | .NET JIT `23.73` |
| `p05_broad_simple_large` | Rust `4.80` | OCaml `14.30` | C `14.32` | .NET JIT `21.88` | .NET AOT `22.04` |
| `p06_row_sweep_complex_large` | OCaml `12.00` | Rust `13.19` | C `31.61` | .NET AOT `71.81` | .NET JIT `73.74` |

All values are milliseconds.

## Which Profile Suits Which Engine Strength

1. OCaml core engine
- Best-fit profiles: `p03`, `p06`, and generally complex formulas with localized/mixed invalidation.
- Why: Incremental propagation + compiled expression cache now handles high-cost formulas efficiently.

2. Rust core engine
- Best-fit profiles: `p02`, `p04`, `p05` and broad/simple workloads.
- Why: consistently low evaluator overhead and strong broad-fanout behavior.

3. C core engine
- Best-fit profiles: broad deterministic throughput baselines (`p01`, `p02`, `p05`), where predictability matters more than incremental locality gains.
- Why: currently less sensitive to locality than OCaml/Rust in this harness family.

4. .NET managed-jit / native-aot (current builds)
- Best-fit use in this profile set is not raw recalc latency leadership.
- Why: in this environment/run family, both .NET variants are dominated by other engines on mean recalc.
- Use these profiles mainly for regression tracking and API parity, not headline throughput.

## Profile Selection Guidance

Use at least one profile from each class below before drawing conclusions:

1. Broad fan-out + complex formula (`p02`): stress total compute throughput.
2. Localized + complex formula (`p03`): stress true incremental recompute.
3. Broad fan-out + simple formula (`p05`): expose scheduler overhead vs evaluator cost.
4. Localized + simple formula (`p04`): lower-bound per-recalc overhead.

Avoid single-profile “winner” claims. Engine rankings can flip across profile classes.

## Repro

```powershell
powershell -ExecutionPolicy Bypass -File scripts/windows/build_coreengines_optimized.ps1
powershell -ExecutionPolicy Bypass -File scripts/windows/run_engine_profile_matrix.ps1 -Label profile_snapshot
```
