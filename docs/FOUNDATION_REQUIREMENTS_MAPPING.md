# Foundation Guidance Mapping (Pathfinder Repo)

This document maps key Foundation guidance to this implementation repo.

## CHARTER alignment
- Clean-room: no proprietary source usage.
- Determinism-first: deterministic eval order and explicit cycle errors.
- Design for evolution: core crate isolated from adapters/UI.

## ARCHITECTURE_AND_REQUIREMENTS alignment
- Core/file/UI boundaries implemented explicitly.
- Profile-like pathfinder subset defined in `docs/SPEC_v0.md`.
- Manual/automatic recalc and stale signaling implemented.

## OPERATIONS alignment (lightweight adaptation)
- Implementation-first rhythm with rapid test/fix loops.
- Evidence in this repo is code + tests + testing logs.
- Regression discipline via added tests and fuzz/property suites.
- Local implementation-run doctrine and artifact structure are defined in `docs/OPERATIONS.md`.

## INDEPENDENT_REVIEW response
- Started from working code and expanded iteratively.
- Kept one implementation language in pathfinder (Rust).
- Built a headless reusable core and layered adapters on top.
- Prioritized testability and deterministic behavior from the start.
