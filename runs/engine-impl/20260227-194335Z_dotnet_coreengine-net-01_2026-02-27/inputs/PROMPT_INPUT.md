# Prompt Input

## Primary Instruction

You are implementing a **new compatible core spreadsheet engine** in C#.

### Objective
Build a **good-faith, high-quality implementation** of the DNA VisiCalc core engine contract with limited context, based only on the frozen spec pack:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`

Target runtime:
- **.NET 10**

API boundary:
- Provide **native C exports** implementing the `dvc_*` C API surface from the spec.

### Constraints
1. Treat the three primary spec docs above as normative.
2. Do not depend on Rust implementation internals; Rust appendix docs are informative only.
3. Keep behavior deterministic for identical inputs/operation sequences.
4. Distinguish success/reject/error status classes exactly as spec'd.
5. Prefer a clear, maintainable architecture over premature optimization.

### What to implement (minimum expected)
1. Engine lifecycle and state/config APIs.
2. Cell and name typed setters/getters, A1 wrappers.
3. Recalc modes, explicit recalc, epoch/stale semantics.
4. Parser/evaluator sufficient for required function surface in SPEC_v0.
5. Dynamic arrays/spill behavior + spill query functions.
6. Structural row/column operations with deterministic reference rewrites.
7. Formatting APIs and deterministic input/name/format iterators.
8. Volatility and invalidation pathways (`invalidate_volatile`, `tick_streams`, `invalidate_udf`).
9. Iteration config APIs and cycle handling behavior.
10. Controls/charts/change-tracking APIs.
11. Utility/diagnostic APIs and UTF-8 buffer contract behavior.
12. Thread-safety contract as specified for handle usage.

### Native export expectations
- Exports must be callable from C ABI consumers.
- Use explicit C-compatible structs/enums/layouts matching the API doc.
- String encoding must follow UTF-8 byte-length protocol exactly.
- Include a small C-side smoke harness (or equivalent interop test) proving exports load and basic calls work.

### Quality bar for this run
This is not the final engine. Prioritize:
- broad spec coverage,
- correctness and explicit behavior,
- actionable diagnostics,
- testability and documented gaps.

If a feature cannot be fully completed in this run, do not fake completeness. Implement the safest partial behavior and document exact gaps in handoff artifacts.

### Required project/output structure
- Implementation root: `engines/dotnet/coreengine-net-01/`
- Keep a clear split between:
  - engine domain logic,
  - C API/native-export adapter,
  - tests.

### Required validation output
Run and record:
- unit/integration tests for core behavior,
- contract tests for representative C API calls,
- at least one end-to-end scenario covering:
  - formulas,
  - structural edit rewrite,
  - recalc + epochs,
  - export boundary call path.

### Definition of done for this run
1. Build succeeds on .NET 10.
2. Exported C API is loadable and basic calls succeed.
3. Substantial subset of spec behavior is implemented and tested.
4. Known gaps are explicitly listed with severity and next steps.
5. Run artifacts are fully updated in this run bundle (manifest, logs, validation, handoff).

## Follow-up Instructions

1. Start by producing a short implementation plan mapped to spec sections.
2. Then execute in small increments with tests after each major slice.
3. Keep `validation/SUMMARY.yaml` and `handoff/HANDOFF.md` current with real status.
