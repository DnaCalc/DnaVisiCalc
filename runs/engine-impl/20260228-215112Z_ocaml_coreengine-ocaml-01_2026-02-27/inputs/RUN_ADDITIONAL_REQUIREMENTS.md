# Run-Specific Additional Requirements

## Runtime and Build Targets

- Runtime target: OCaml (`ocaml`), implementation root `engines/ocaml/coreengine-ocaml-01`.
- Baseline toolchain: OCaml 5.2.x, dune, opam-managed dependencies.
- Deliverable must include a Windows-loadable DLL exporting the `dvc_*` C API symbols required by `ENGINE_API.md`.

## API and Behavior Constraints

- No hidden behavior outside the C API contract.
- Respect tri-state/diagnostic semantics (`ok`/`reject`/`error`) exactly as specified.
- Reject unsupported operations explicitly; do not silently no-op where contract requires structured failure.
- Preserve deterministic outcomes for identical operation sequences.

## Recalc and Incremental Strategy

- Preferred: use Jane Street `Incremental` for dependency/invalidation/recalc orchestration.
- Acceptable fallback: deterministic non-Incremental scheduler only if blocker is documented with concrete evidence in `execution/SESSION_LOG.md` and `handoff/HANDOFF.md`.
- External behavior must remain spec-conforming regardless of internal strategy.

## Test and Validation Requirements

- Add API-level tests covering create/destroy, cell/name mutation, recalc modes, structural ops, dynamic arrays, stream behavior, diagnostics, and entities (controls/charts/UDF/journal).
- Record exact commands and results under `validation/`.
- Include explicit residual-risk notes for any unimplemented or partially implemented spec requirement.

## Non-goals for This Run

- No TUI changes.
- No file-format adapter implementation work.
- No direct edits to Rust or .NET engine implementation trees.
