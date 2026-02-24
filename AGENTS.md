# AGENTS.md - DNA VisiCalc Execution Guide

This repository is the small-scope Rust pathfinder for DNA Calc.

## 1. Context Loading (required)
Load context in this order before making architectural or scope changes:
1. `README.md` (this repo)
2. `docs/ARCHITECTURE.md` (this repo)
3. `docs/SPEC_v0.md` (this repo)
4. `docs/FOUNDATION_REQUIREMENTS_MAPPING.md` (this repo)
5. `docs/testing/TESTING_PLAN.md` and `docs/testing/TESTING_ROUNDS.md` (this repo)
6. `../Foundation/CHARTER.md`
7. `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
8. `../Foundation/OPERATIONS.md`
9. `../Foundation/notes/INDEPENDENT_REVIEW.md`
10. `../Foundation/notes/RESEARCH_NOTES.md` and `../Foundation/notes/BRAINSTORM_NOTES.md` (supporting)

## 2. Scope Rules for DnaVisiCalc
- Keep this project implementation-first and small.
- Maintain clean crate boundaries:
  - `dnavisicalc-core` (engine library only),
  - `dnavisicalc-file` (serialization adapter),
  - `dnavisicalc-tui` (interaction layer).
- Keep deterministic behavior and reproducible tests as default.
- Prefer lightweight process artifacts tied to executable tests and docs.

## 3. Source-of-Truth and Precedence
When docs conflict for this repository, precedence is:
1. `docs/SPEC_v0.md`
2. `docs/ARCHITECTURE.md`
3. `README.md`
4. `docs/FOUNDATION_REQUIREMENTS_MAPPING.md`
5. `../Foundation/CHARTER.md`
6. `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
7. `../Foundation/OPERATIONS.md`
8. `../Foundation/notes/INDEPENDENT_REVIEW.md`

## 4. Clean-Room and Evidence
- Follow Foundation clean-room doctrine: only public documentation, published research, and reproducible observations.
- Keep compatibility assumptions and evidence links in local docs.

## 5. Change Discipline
- Keep changes minimal, testable, and explicit.
- Add tests with behavior changes.
- Keep hardening/test logs current when deep validation runs are executed.