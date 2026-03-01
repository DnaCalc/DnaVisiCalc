# Issue Artifacts

## Trigger

Harness run against OCaml engine DLL:

- `DNAVISICALC_COREENGINE=dotnet-core`
- `DNAVISICALC_COREENGINE_DLL=C:\Work\DnaCalc\DnaVisiCalc\engines\ocaml\coreengine-ocaml-01\dist\dvc_coreengine_ocaml01.dll`
- Command:
  - `cargo test -p dnavisicalc-engine --test conformance_smoke`

Observed result:

- `running 13 tests`
- `FAILED. 11 passed; 2 failed`

Failing tests:

1. `ct_entities_001_control_roundtrip_holds`
   - assertion failure:
     - `control iterator length mismatch`
     - `left: 1`, `right: 2`

2. `ct_entities_002_chart_roundtrip_holds`
   - assertion failure:
     - `chart iterator length mismatch`
     - `left: 0`, `right: 1`

## Expected Behavior

Per `ENGINE_API.md`, entity iteration APIs must enumerate existing definitions deterministically:

- `dvc_control_iterate` / `dvc_control_iterator_next` must report all controls.
- `dvc_chart_iterate` / `dvc_chart_iterator_next` must report all charts.

Roundtrip tests expect that defining entities and reading them through iterators is internally consistent.

## Run Goal

Fix the entity management behavior so these harness tests pass without regressing existing CT/API behavior.
