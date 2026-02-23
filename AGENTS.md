# AGENTS.md - DNA VisiCalc Execution Guide

This repository is the small-scope Rust pathfinder for DNA Calc.

## 1. Context Loading (required)
Load context in this order before making architectural or scope changes:
1. `README.md` (this repo)
2. `docs/SPEC_v0.md` (this repo)
3. `../Foundation/CHARTER.md`
4. `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
5. `../Foundation/OPERATIONS.md`
6. `../Foundation/notes/INDEPENDENT_REVIEW.md`
7. `../Foundation/notes/RESEARCH_NOTES.md` and `../Foundation/notes/BRAINSTORM_NOTES.md` (supporting)

## 2. Scope Rules for DnaVisiCalc
- Keep this project implementation-first and small.
- Rust core engine library only (no UI, no file I/O, no network I/O).
- Prioritize deterministic behavior, reproducible tests, and clean APIs.
- Prefer one strong implementation over process-heavy scaffolding.

## 3. Source-of-Truth and Precedence
When docs conflict for this repository, precedence is:
1. `docs/SPEC_v0.md`
2. `README.md`
3. `../Foundation/CHARTER.md`
4. `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
5. `../Foundation/OPERATIONS.md`
6. `../Foundation/notes/INDEPENDENT_REVIEW.md`

## 4. Clean-Room and Evidence
- Follow Foundation clean-room doctrine: only public documentation, published research, and reproducible observations.
- Track historical compatibility assumptions and references in local docs.

## 5. Change Discipline
- Keep changes minimal, testable, and explicit.
- Add tests with behavior changes.
- Do not add heavyweight process artifacts unless directly justified by implementation needs.