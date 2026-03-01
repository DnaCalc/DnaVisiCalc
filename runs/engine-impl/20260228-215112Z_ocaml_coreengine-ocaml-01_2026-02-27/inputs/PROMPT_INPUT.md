# Prompt Input

## Primary Instruction

You are executing a clean-room compatible core-engine implementation run for:
- runtime: `ocaml`
- implementation id: `coreengine-ocaml-01`
- target codebase root: `engines/ocaml/coreengine-ocaml-01`
- pinned spec pack: `docs/full-engine-spec/2026-02-27`

Normative contract documents:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`

Context constraints:
- Implement from the pinned spec pack only.
- Do not read existing Rust or .NET engine implementations.
- Do not infer behavior from adapter/TUI code.
- If spec text is ambiguous, choose the most conservative interpretation, document it, and proceed.

Primary objective:
- Implement a DLL-exported C API core engine in OCaml that is drop-in compatible with the DNA VisiCalc engine boundary according to the pinned spec pack.

Required delivery outcomes:
1. Implement the required `dvc_*` API surface and status/outcome semantics in `ENGINE_API.md`.
2. Implement deterministic workbook/cell/name/formula behavior in the required scope from `SPEC_v0.md` and `ENGINE_REQUIREMENTS.md`.
3. Implement dynamic arrays/spill behavior, structural row/column operations, and reference rewrite semantics (including mixed absolute references).
4. Implement recalc/epoch/staleness semantics, manual/automatic mode behavior, explicit recalc pathway, and invalidation classes.
5. Implement cycle behavior per spec (including non-fatal circular diagnostics in non-iterative mode).
6. Implement function coverage in scope, including `LET`, `LAMBDA`, `MAP`, `SEQUENCE`, `RANDARRAY`, `STREAM`, `INDIRECT`, and `OFFSET`.
7. Implement controls/charts/UDF/change-journal APIs to contract (not silent stubs).
8. Produce a buildable native DLL artifact with exported `dvc_*` symbols suitable for host loading in later integration tests.
9. Provide tests and validation evidence mapped to requirements and API surface.

Implementation guidance:
- Target OCaml 5.2.x + dune build system.
- Use Jane Street `Incremental` to drive recalculation/dependency invalidation if feasible on this toolchain.
- If `Incremental` is not feasible due to environment/toolchain constraints, document concrete blocker evidence and implement a deterministic fallback that still satisfies the same observable API behavior.
- Keep the export layer thin; keep engine logic separated from FFI glue.
- Keep behavior deterministic for identical inputs + API call sequences.

## Follow-up Instructions

1. Start with a spec-to-implementation closure plan in `execution/SESSION_LOG.md` mapping requirements/API groups to code and tests.
2. Implement in small slices and update `execution/TOOL_LOG.jsonl` with command traces and path-access policy outcomes.
3. Keep `validation/COMMANDS.md`, `validation/RESULTS.md`, and `validation/SUMMARY.yaml` current during the run.
4. Track unresolved gaps explicitly in `handoff/HANDOFF.md` with blocker evidence and next actions.
5. Do not mark the run complete unless build/export/test evidence is recorded in validation artifacts.
