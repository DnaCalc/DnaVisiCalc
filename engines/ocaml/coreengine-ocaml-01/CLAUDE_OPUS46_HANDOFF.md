# Claude Opus 4.6 Handoff: coreengine-ocaml-01

You are implementing a production-quality OCaml engine with full conformance.

## Scope

Work in:
- `engines/ocaml/coreengine-ocaml-01/**`

Primary spec inputs:
1. `src/dvc_engine.h`
2. `tests/api_smoke.c`
3. `tests/api_closure.c`
4. `tests/api_conformance_ct.c`
5. `crates/dnavisicalc-engine/tests/conformance_smoke.rs`

## Hard Constraints

- OCaml-owned logic with Jane Street Incremental.
- Thin C bridge in `src/dvc_engine.c` only.
- No wholesale reuse of `engines/c/coreengine-c-01/src/dvc_engine.c` as final implementation.

## Required Outcome

Pass all:
- `dist/api_smoke.exe`
- `dist/api_closure.exe`
- `dist/api_conformance_ct.exe`
- `cargo test -p dnavisicalc-engine --test conformance_smoke` with
  - `DNAVISICALC_COREENGINE=dotnet-core`
  - `DNAVISICALC_COREENGINE_DLL=<absolute path to dist/dvc_coreengine_ocaml01.dll>`

## Suggested Execution Loop

1. Reconstruct OCaml module interfaces/logic.
2. Implement thin C callbacks and exports.
3. Build DLL.
4. Run smoke/closure/CT tests.
5. Fix failures iteratively until green.
6. Run Rust conformance smoke and report results.

## Output Format

At completion, report:
- Changed files
- Build/test commands
- Final pass matrix
- Notes on Incremental architecture choices
