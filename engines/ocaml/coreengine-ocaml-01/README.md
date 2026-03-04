# coreengine-ocaml-01

This directory has been reset to a clean scaffold for a true OCaml implementation.

## What Is Preserved

- `src/dvc_engine.h`: C API contract surface.
- `tests/api_smoke.c`: executable API smoke checks.
- `tests/api_closure.c`: closure/conformance behavior checks.
- `tests/api_conformance_ct.c`: CT-* conformance checks.
- `src/dune`, `tests/dune`, `dune-project`: build scaffolding.

## Reset State

- OCaml implementation modules under `src/*.ml` are placeholders.
- `src/dvc_engine.c` is a reset marker and must be replaced with a thin C FFI bridge.
- Previous implementation artifacts (`dist/`, `_build/`, object files) were removed.

## Required Direction

Implement a best-in-class OCaml engine (Jane Street Incremental) with:

- OCaml-owned core logic.
- Thin C FFI bridge only.
- Full pass of smoke + closure + CT conformance suites.

See `BEST_OCAML_IMPLEMENTATION_REQUIREMENTS.md` and `CLAUDE_OPUS46_HANDOFF.md`.

## Build (Optimized DLL)

Use:

`build_release.cmd`

This script performs a deterministic native build pipeline:
- `dune build`
- `ocamlfind ocamlopt ... -output-complete-obj` (with explicit MinGW `-L` flags)
- `gcc` shared link of `dist/dvc_coreengine_ocaml01.dll`
- build + run `api_smoke`, `api_closure`, `api_conformance_ct`
