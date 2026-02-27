# Context for Compatible Core Engine

## 1. Objective

Implement a core spreadsheet engine that is functionally drop-in compatible with the DNA VisiCalc engine boundary.

## 2. What Compatibility Means

- Same observable behavior for the supported core scope.
- Same status/outcome semantics at the API boundary.
- Same deterministic behavior for identical inputs and operation sequences.

## 3. Contract Documents

Primary contract docs in this pack:
- `SPEC_v0.md`
- `ENGINE_REQUIREMENTS.md`
- `ENGINE_API.md`

These are copied from the canonical `docs/` versions and should be treated as the source of truth for implementation behavior.

## 4. Normative vs Informative Material

Normative for core implementation:
- all content in `SPEC_v0.md`,
- all REQ items in `ENGINE_REQUIREMENTS.md`,
- API contract sections in `ENGINE_API.md`.

Informative/companion only (not required for core contract compliance):
- `SPEC_v0_INTEGRATION_APPENDIX.md` (repo adapter/UI scope),
- `ENGINE_REQUIREMENTS_INTEGRATION_APPENDIX.md` (integration handoff requirement),
- `ENGINE_API_RUST_APPENDIX.md` (Rust mapping and Rust-specific implementation notes).

## 5. Out of Scope for This Implementation Task

- UI rendering/input loop behavior.
- File adapter parser/writer implementation.
- Development process/doctrine guidance.
- Language-specific implementation style.
