# Conformance and Performance Plan

## 1. Purpose
Define a repeatable, evidence-oriented run plan for:
- cross-engine conformance (`rust-core`, `.NET` C API backends),
- baseline performance signatures tied to the same scenarios.

## 2. Execution Matrix
Each run is a matrix over:
- engine backend (`DNAVISICALC_COREENGINE`, optional explicit DLL path),
- conformance case ID (`CT-*`),
- run tier (`quick`, `full`, `release`).

## 3. Run Tiers
- `quick` (PR/local):
  - mandatory smoke conformance cases,
  - no perf gating, metrics optional.
- `full` (nightly/merge hardening):
  - all mandatory conformance cases,
  - temporal property cases included.
- `release` (stabilization candidate):
  - full conformance pass,
  - recorded performance signatures and diff against baseline.

## 4. Initial Case Sources
- `crates/dnavisicalc-engine/tests/conformance_smoke.rs`
- `crates/dnavisicalc-core/tests/conformance_invariants_stub.rs` (placeholder backlog seam)
- `.NET` interop/core tests under `engines/dotnet/coreengine-net-01/tests/*` for C API parity.

## 5. Performance Signature Scope (Non-Gating Initially)
For each backend, record:
- end-to-end wall time for selected conformance scenarios,
- recalc-focused micro-scenarios (manual recalc, structural edit + rewrite, dynamic-array recompute),
- scenario-size metadata (input counts, formula counts, structural op count).

Initial policy:
- collect and report only,
- no hard thresholds until enough baseline history exists.

## 6. Artifact Layout
Conformance/perf runs should emit:
- `runs/conformance/<run-id>/report.json`
- `runs/conformance/<run-id>/summary.md`
- `runs/conformance/<run-id>/perf.json` (when perf captured)

Each report includes backend identity (engine id, DLL path/hash if available) and per-case outcomes (`pass`/`fail`/`waived`).

## 7. Current Recalc Perf Harness
- Binary: `cargo run -p dnavisicalc-engine --bin engine_perf_compare -- ...`
- Output artifact example: `.tmp/perf/engine_recalc_compare_latest.txt`
- Key knobs:
  - `--iterations <n>`
  - `--formula-cols <n>`
  - `--formula-rows <n>`
  - `--full-data <true|false>`

This harness is intended to stress engine recalc behavior under dense formula dependency regions, while keeping API-call overhead out of the timed recalc loop.
