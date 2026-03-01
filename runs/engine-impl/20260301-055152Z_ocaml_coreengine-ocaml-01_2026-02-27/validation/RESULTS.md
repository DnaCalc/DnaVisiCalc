# Validation Results

Record command outputs or concise summaries.

- command: `cargo test -p dnavisicalc-engine --test conformance_smoke` (pre-fix baseline)
  status: fail
  notes: Reproduced reported failures exactly: `ct_entities_001_control_roundtrip_holds` (`left: 1`, `right: 2`) and `ct_entities_002_chart_roundtrip_holds` (`left: 0`, `right: 1`).

- command: `cargo test -p dnavisicalc-engine --test conformance_smoke ct_entities_001_control_roundtrip_holds -- --nocapture`
  status: fail
  notes: With temporary trace enabled, engine showed both controls were defined and surfaced by iterator; this isolated the issue to iterator consumption semantics under probe-then-fetch caller usage.

- command: `cargo test -p dnavisicalc-engine --test conformance_smoke` (post-fix, pinned DLL)
  status: pass
  notes: `running 13 tests` -> `13 passed; 0 failed`.

- command: `./dist/api_conformance_ct.exe`
  status: pass
  notes: Local OCaml conformance binary passed existing and new entity regression tests (`CT-ENTITIES-001`, `CT-ENTITIES-002`).

- command: `x86_64-w64-mingw32-nm -g engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll | rg " T dvc_" | Measure-Object | Select-Object -ExpandProperty Count`
  status: pass
  notes: Exported `dvc_*` symbol count is `104` (unchanged from prior baseline expectation).
