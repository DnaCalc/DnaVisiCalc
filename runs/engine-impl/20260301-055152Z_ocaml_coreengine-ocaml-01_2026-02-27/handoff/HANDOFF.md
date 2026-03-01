# Handoff

## Outcome

- status: completed
- implementation: coreengine-ocaml-01
- spec_pack: 2026-02-27

## What Was Implemented

- Fixed control/chart iterator consumption semantics in `engines/ocaml/coreengine-ocaml-01/src/dvc_engine.c`:
  - `dvc_control_iterator_next`
  - `dvc_chart_iterator_next`
- Iterators now advance only after successful name-copy calls; length-probe calls (`name_buf == NULL`) no longer consume entries.
- Added OCaml-owned regression coverage for this pattern in `engines/ocaml/coreengine-ocaml-01/tests/api_conformance_ct.c`:
  - `test_ct_entities_001`
  - `test_ct_entities_002`
- Rebuilt `dist/dvc_coreengine_ocaml01.dll` and verified pinned harness command passes.

## Open Items

- None within this run scope.

## Next Suggested Run

- Optional: expand OCaml-local CT matrix to include additional entity edge cases (buffer-too-small retry paths and mixed-case name ordering).
