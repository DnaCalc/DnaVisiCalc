# API Pending Matrix (Reset Baseline)

Current status: reset scaffold only.

- Implemented behavior: none (runtime/engine logic intentionally cleared).
- Preserved specs/tests: all required conformance test sources in `tests/`.
- Next target: full conformance via OCaml-owned engine implementation.

Authoritative behavior target is defined by:

1. `src/dvc_engine.h`
2. `tests/api_smoke.c`
3. `tests/api_closure.c`
4. `tests/api_conformance_ct.c`
5. `crates/dnavisicalc-engine/tests/conformance_smoke.rs`
