# Prompt Input

## Primary Instruction

You are executing a full-conformance completion run for:
- runtime: `ocaml`
- implementation id: `coreengine-ocaml-01`
- target codebase root: `engines/ocaml/coreengine-ocaml-01`
- pinned spec pack: `docs/full-engine-spec/2026-02-27`

Normative contract docs:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_CONFORMANCE_TESTS.md`

Previous run context:
- `runs/engine-impl/20260228-215112Z_ocaml_coreengine-ocaml-01_2026-02-27/handoff/HANDOFF.md`
- `runs/engine-impl/20260228-222210Z_ocaml_coreengine-ocaml-01_2026-02-27/handoff/HANDOFF.md`
- `runs/engine-impl/20260228-222210Z_ocaml_coreengine-ocaml-01_2026-02-27/validation/SUMMARY.yaml`

Objective:
- Bring `coreengine-ocaml-01` to full practical conformance for the current v0 contract, with explicit CT-ID evidence.

Required outcomes for this run:
1. Close remaining high-impact function-surface gaps against required v0 list.
2. Expand structural rewrite correctness and rejection-context behavior to contract-level consistency.
3. Ensure cycle/non-iterative diagnostics and invalidation/recalc pathways align with spec.
4. Add and run requirement-mapped conformance tests covering at least:
   - `CT-EPOCH-001`, `CT-EPOCH-002`, `CT-CELL-001`, `CT-DET-001`, `CT-STR-001`, `CT-CYCLE-001`.
5. Produce a conformance matrix artifact (pass/fail by CT-ID and key REQ groups).
6. Preserve C API export compatibility (`dvc_*`) and working DLL.

Completion rule:
- Mark run `completed` only if conformance matrix and validation evidence substantiate the claim.
- Otherwise mark `blocked` with explicit remaining gaps and next closure slice.

## Follow-up Instructions

1. Start with an explicit CT/REQ closure plan in `execution/SESSION_LOG.md`.
2. Implement and test incrementally; keep `execution/TOOL_LOG.jsonl` and validation docs current.
3. Finalize run artifacts truthfully (`RUN_MANIFEST`, validation, outputs, handoff).
