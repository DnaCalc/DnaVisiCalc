# Prompt Input

## Primary Instruction

You are executing a second implementation run to complete `coreengine-net-01` to full conformance with the frozen spec pack:
- `docs/full-engine-spec/2026-02-27/SPEC_v0.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_REQUIREMENTS.md`
- `docs/full-engine-spec/2026-02-27/ENGINE_API.md`

Current codebase:
- `engines/dotnet/coreengine-net-01`

Previous run handoff (gap baseline):
- `runs/engine-impl/20260227-194335Z_dotnet_coreengine-net-01_2026-02-27/handoff/HANDOFF.md`

Objective:
- Close all identified gaps and deliver a full-spec implementation for this round.
- If any requirement still cannot be completed, mark it explicitly as blocked with concrete reason and failing test evidence.

Mandatory outcomes:
1. Complete the full `dvc_*` API behavior surface as specified (or clearly blocked with evidence).
2. Upgrade formula/evaluator coverage to match required function/reference behavior scope in spec docs.
3. Complete dynamic array/spill semantics including spill references and range interactions.
4. Complete deterministic structural rewrite semantics for row/column operations, including mixed absolute references.
5. Complete volatility and invalidation pathway behavior per spec.
6. Complete iteration/cycle behavior per spec contract.
7. Complete controls/charts/change-tracking/UDF surfaces per API contract.
8. Ensure UTF-8 buffer protocols and null-pointer validation are correct for all applicable APIs.
9. Produce and verify native exports (`dvc_*`) in published artifact.
10. Expand tests to cover full scope with explicit pass/fail evidence and residual risk summary.

Constraints:
- Keep architecture maintainable with thin export layer and core logic separated.
- No silent stub/no-op pretending completeness.
- Deterministic behavior for identical inputs/operation sequences.
- Distinguish status classes (`ok`, `reject`, `error`) according to spec.

## Follow-up Instructions

1. Start with a spec-mapped closure plan listing each previous handoff gap and target test evidence.
2. Implement in slices, running tests after each slice; keep run artifacts current.
3. Finish only when validation and handoff reflect real completion status.
