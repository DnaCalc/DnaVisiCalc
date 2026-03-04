# coreengine-c-01

This directory is a pure C copy of the previous `coreengine-ocaml-01` engine
surface.

Current status:
- C API implementation lives in `src/dvc_engine.c` and `src/dvc_engine.h`.
- C smoke/conformance tests live in `tests/*.c`.
- No OCaml wrapper/module is included in this directory.

Planned output artifact:
- `dist/dvc_coreengine_c01.dll`

Notes:
- This is intentionally a low-change fork to preserve behavior while the
  original OCaml directory is repurposed for a true OCaml implementation.
- Catalog target id: `c/coreengine-c-01/native/release`.
