# Prompt Input

## Primary Instruction

You are executing a managed sandbox bug-fix run for:
- runtime: `ocaml`
- implementation id: `coreengine-ocaml-01`
- target codebase root: `engines/ocaml/coreengine-ocaml-01`
- pinned spec pack: `docs/full-engine-spec/2026-02-27`

Normative docs:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`

Issue artifact (must drive this run):
- `runs/engine-impl/20260301-055152Z_ocaml_coreengine-ocaml-01_2026-02-27/inputs/ISSUE_ARTIFACTS.md`

Objective:
- Fix the control/chart management non-conformance reported in the issue artifact.
- Validate the fix using the recorded harness command and preserve existing export surface/API behavior.

Constraints:
- Use only spec docs + issue artifact + owned engine tree.
- Do not read Rust/.NET engine implementation trees.
- Keep edits minimal and behavior-focused.

Completion rule:
- Mark `completed` only if the recorded failing harness tests are now passing with evidence.
- If still failing, mark `blocked` with updated diagnostics and narrowed root-cause notes.

## Follow-up Instructions

1. Start by mapping issue -> spec requirement in `execution/SESSION_LOG.md`.
2. Implement a focused fix in `engines/ocaml/coreengine-ocaml-01`.
3. Run the recorded harness command against the OCaml DLL.
4. Update all run artifacts truthfully (`validation/*`, `handoff/HANDOFF.md`, `RUN_MANIFEST.yaml`).
