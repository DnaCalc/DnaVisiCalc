# Run-Specific Additional Requirements

These requirements refine this run beyond the baseline template.

## 1. Runtime and Tooling

- Target SDK/runtime: `.NET 10`.
- If `.NET 10` is unavailable in the environment, record blocker in `validation/SUMMARY.yaml` and proceed with closest preview/runtime only if explicitly documented.

## 2. Native Export Strategy

- Native exports must be explicit and discoverable as `dvc_*` symbols.
- Include a concise design note in code/docs explaining export mechanism choice.
- Keep exported boundary thin; avoid embedding business logic in export layer.

## 3. Interop Robustness

- Validate struct layout and enum values against `ENGINE_API.md`.
- Include tests for null-pointer handling and buffer-length query pattern.

## 4. Run Artifact Completeness

Before closing run:
- fill `inputs/INPUT_HASHES.json`,
- fill `outputs/CODEBASE_REF.yaml`,
- fill `outputs/OUTPUT_HASHES.json`,
- ensure `validation/RESULTS.md` and `validation/SUMMARY.yaml` reflect actual run state.

## 5. Gap Reporting Standard

`handoff/HANDOFF.md` must classify open items as:
- `missing-feature`,
- `behavior-mismatch-risk`,
- `interop-risk`,
- `performance-risk`.
