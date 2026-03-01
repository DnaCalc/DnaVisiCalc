# Prompt Input

## Primary Instruction

You are executing a continuation implementation run for:
- runtime: `ocaml`
- implementation id: `coreengine-ocaml-01`
- target codebase root: `engines/ocaml/coreengine-ocaml-01`
- pinned spec pack: `docs/full-engine-spec/2026-02-27`

Normative contract docs:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`

Previous run context (input-only):
- `runs/engine-impl/20260228-215112Z_ocaml_coreengine-ocaml-01_2026-02-27/handoff/HANDOFF.md`
- `runs/engine-impl/20260228-215112Z_ocaml_coreengine-ocaml-01_2026-02-27/validation/SUMMARY.yaml`

Objective:
- Progress `coreengine-ocaml-01` toward conformance by closing the highest-impact behavioral gaps identified in the previous blocked run.
- This is an iterative closure run; do not attempt to over-claim completion.

Required closure focus for this run:
1. Improve formula/function behavior coverage for required v0 functions, prioritizing:
   - `LET`, `LAMBDA`, `MAP`, `SEQUENCE`, `RANDARRAY`, `STREAM`, `INDIRECT`, `OFFSET`
   - plus direct dependency and recalc behavior for these functions.
2. Improve structural operations semantics:
   - row/column insert/delete behaviors,
   - reference adjustments including `$A$1`, `$A1`, `A$1`, `A1`,
   - reject-vs-invalid classification and last-reject context behavior.
3. Improve cycle/non-iterative diagnostics behavior:
   - preserve non-fatal handling,
   - emit observable diagnostic/change-journal evidence where required.
4. Expand requirement-mapped tests and validation evidence beyond smoke level.
5. Keep DLL export surface intact and runnable.

Constraints:
- Implement from pinned spec docs only.
- Do not read Rust/.NET engine implementations.
- Be explicit about what is still not conformant.
- If blocked on scope/time, leave the run `blocked` with precise remaining gap list.

## Follow-up Instructions

1. Start with a short closure plan in `execution/SESSION_LOG.md` mapped to REQ/API IDs.
2. Implement and test in slices; update `execution/TOOL_LOG.jsonl` continuously.
3. Keep `validation/COMMANDS.md` and `validation/RESULTS.md` synchronized with actual command output.
4. Finalize run artifacts truthfully (`RUN_MANIFEST`, `validation/SUMMARY.yaml`, `handoff/HANDOFF.md`, output hashes).
