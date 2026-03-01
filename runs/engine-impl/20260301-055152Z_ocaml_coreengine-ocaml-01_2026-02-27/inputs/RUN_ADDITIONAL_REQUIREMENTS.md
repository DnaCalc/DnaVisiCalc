# Run-Specific Additional Requirements

## Scope

- Bug-fix run only: control/chart management conformance issue.
- No broad refactor or unrelated feature expansion.

## Required Validation

- Must rerun:
  - `cargo test -p dnavisicalc-engine --test conformance_smoke`
  - with `DNAVISICALC_COREENGINE=dotnet-core`
  - and `DNAVISICALC_COREENGINE_DLL` pinned to OCaml DLL path.
- Target: previously failing cases pass:
  - `ct_entities_001_control_roundtrip_holds`
  - `ct_entities_002_chart_roundtrip_holds`

## Safety Constraints

- Preserve `dvc_*` export count and existing passing CT behavior.
- Keep fix localized to ownership scope.
