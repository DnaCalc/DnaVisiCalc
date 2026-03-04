# Best OCaml Implementation Requirements

## Mandatory Architecture

- Core engine behavior must be implemented in OCaml modules.
- Use Jane Street `Incremental` as the recalculation/runtime foundation.
- `src/dvc_engine.c` must be a thin FFI/export bridge only.
- Do not embed business logic in C.

## Quality Bar

- Deterministic behavior across runs for identical inputs.
- Clear module architecture (parser, eval, dependency graph, state, bridge).
- Explicit error/reject handling and diagnostic propagation.
- Efficient incremental recomputation strategy.

## Compliance Targets (must all pass)

1. `dist/api_smoke.exe`
2. `dist/api_closure.exe`
3. `dist/api_conformance_ct.exe`
4. `cargo test -p dnavisicalc-engine --test conformance_smoke`

## Forbidden Shortcuts

- No wholesale copy of pure-C engine implementation as final engine.
- No C-first fallback where OCaml modules are bypassed.
- No stubbing to satisfy tests without real behavior.

## Deliverables

- Working OCaml engine DLL at `dist/dvc_coreengine_ocaml01.dll`.
- Passing outputs for all compliance targets.
- Short implementation report with architecture notes and command log.
