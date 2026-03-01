# Prompt Input

## Primary Instruction

You are executing a managed-context implementation run for:
- runtime: `dotnet`
- implementation id: `coreengine-net-01`
- target path: `engines/dotnet/coreengine-net-01`
- spec pack: `docs/full-engine-spec/2026-02-27`

Goal:
- Perform an internal performance-oriented refactor of the .NET core engine so recalc behavior is in the same general ballpark as the Rust and OCaml engines for small/moderate workloads.
- Keep behavior/spec compatibility intact (no intentional semantic regressions).

Hard constraints:
1. Do not change repository code outside:
   - `engines/dotnet/coreengine-net-01/**`
   - `runs/engine-impl/20260301-075400Z_dotnet_coreengine-net-01_2026-02-27/**`
2. Use the spec docs as normative behavior source:
   - `SPEC_v0.md`
   - `ENGINE_REQUIREMENTS.md`
   - `ENGINE_API.md`
3. Prefer refactors that improve scaling properties for larger inputs:
   - avoid full-sheet recompute where possible,
   - avoid repeated parse/eval work when inputs are unchanged,
   - preserve deterministic observable behavior.
4. Keep C API contract and export surface unchanged unless required by spec or correctness.
5. Update/extend tests in the .NET engine tree to lock in both correctness and performance-sensitive behavior assumptions.

Performance direction:
- Use `docs/testing/CONFORMANCE_PERFORMANCE_PLAN.md` and `crates/dnavisicalc-engine/src/bin/engine_perf_compare.rs`.
- Baseline evidence is in `inputs/ISSUE_ARTIFACTS.md`.
- Minimum acceptance for this run:
  - substantial improvement over baseline in recalc microbenchmarks (target >= 10x better on the baseline tiny workload),
  - no conformance-smoke regressions under backend-pinned execution.
- Stretch target:
  - move toward Rust/OCaml ballpark on tiny workloads (single-digit ms recalc, ideally low-ms/sub-ms).

Required outputs:
- Implemented refactor in `engines/dotnet/coreengine-net-01/**`.
- Updated run artifacts:
  - `execution/SESSION_LOG.md`
  - `execution/TOOL_LOG.jsonl`
  - `validation/COMMANDS.md`
  - `validation/RESULTS.md`
  - `validation/SUMMARY.yaml`
  - `handoff/HANDOFF.md`
  - `outputs/CODEBASE_REF.yaml`
  - `outputs/OUTPUT_HASHES.json`
- Include concrete before/after benchmark numbers and exact commands.

## Follow-up Instructions

1. If a required behavior is unclear in spec text, document the ambiguity and choose the most conservative spec-consistent behavior.
2. Prefer algorithmic improvements over superficial micro-optimizations.
