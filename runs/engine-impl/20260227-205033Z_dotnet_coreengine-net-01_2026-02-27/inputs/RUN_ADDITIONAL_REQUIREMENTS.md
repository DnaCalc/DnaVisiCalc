# Run-Specific Additional Requirements

This run is a **full-spec completion pass** for `coreengine-net-01`.

## 1. Runtime and Tooling

- Target SDK/runtime: `.NET 10`.
- If exact `.NET 10` toolchain is unavailable, stop and mark run `blocked`.

## 2. Completion Bar

- Previous gap classes (`missing-feature`, `behavior-mismatch-risk`, `interop-risk`, `performance-risk`) from the prior run must be either:
  - resolved and covered by tests, or
  - explicitly blocked with concrete reason plus failing/omitted test evidence.
- Do not close run as `completed` with unresolved gaps lacking evidence.

## 3. API Coverage

- Ensure all exported `dvc_*` functions in `ENGINE_API.md` are implemented with contract-appropriate behavior.
- For unsupported paths, return explicit status and diagnostic details per spec (no silent no-op behavior).

## 4. Conformance Evidence

- `validation/RESULTS.md` must include:
  - expanded function-surface test outcomes,
  - structural rewrite edge-case outcomes (`$A$1`, `$A1`, `A$1`),
  - spill-reference outcomes (`A1#`) and spill/range interactions,
  - interop buffer/length and null-pointer outcomes across representative APIs.
- `handoff/HANDOFF.md` must clearly separate:
  - resolved prior gaps,
  - remaining blockers (if any), with rationale and next action.

## 5. Artifact Quality

- Ensure published native artifact exists and export symbols are verifiable.
- Keep run docs synchronized with actual executed commands and results.
