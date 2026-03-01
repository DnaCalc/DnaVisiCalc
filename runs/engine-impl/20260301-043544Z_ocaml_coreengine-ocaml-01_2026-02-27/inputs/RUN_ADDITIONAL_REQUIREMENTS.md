# Run-Specific Additional Requirements

## Run Type

- Full-conformance completion attempt for current v0 spec pack.
- This run should produce a CT-ID conformance matrix, not only smoke output.

## Mandatory Evidence

- Include executable tests mapped to:
  - `CT-EPOCH-001`
  - `CT-EPOCH-002`
  - `CT-CELL-001`
  - `CT-DET-001`
  - `CT-STR-001`
  - `CT-CYCLE-001`
- Persist matrix artifact under this run bundle with pass/fail + evidence pointers.

## Implementation Constraints

- Keep all edits within ownership scope:
  - `engines/ocaml/coreengine-ocaml-01/**`
  - `runs/engine-impl/20260301-043544Z_ocaml_coreengine-ocaml-01_2026-02-27/**`
- Do not break existing DLL build and export surface.
- Maintain deterministic behavior and explicit reject/error semantics.

## Open-Scope Clarification

- Full mathematical/financial parity beyond currently required v0 set is out of scope.
- If unresolved gaps remain after this pass, quantify them exactly by REQ/CT ID.
