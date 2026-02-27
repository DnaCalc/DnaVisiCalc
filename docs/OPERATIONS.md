# Local Operations Guide (DnaVisiCalc)

This guide defines the local execution doctrine for implementation runs in this repository.

It complements, but does not replace, `../Foundation/OPERATIONS.md`.

## 1. Purpose

The goal is reproducible, auditable implementation runs for multiple compatible core engines (Rust/.NET and future variants), with structured capture of:
- prompt inputs,
- execution artifacts,
- validation outputs,
- resulting codebase state.

## 2. Scope

This guide applies to:
- engine implementation runs from frozen spec packs,
- review/hardening runs against an existing implementation,
- cross-implementation parity runs.

## 3. Canonical Directory Layout

### 3.1 Implementation code roots

Compatible engine implementations live under:
- `engines/rust/<implementation-id>/`
- `engines/dotnet/<implementation-id>/`

Examples:
- `engines/rust/coreengine-rs-01/`
- `engines/dotnet/coreengine-net-01/`

### 3.2 Run bundles

Each run gets an immutable bundle directory:
- `runs/engine-impl/<run-id>/`

`<run-id>` format:
- `YYYYMMDD-HHMMSSZ_<runtime>_<implementation-id>_<spec-pack-version>`

Example:
- `20260228-091500Z_dotnet_coreengine-net-01_2026-02-27`

### 3.3 Temporary/heavy artifacts

Large/transient artifacts must use repo-local temp:
- `.tmp/runs/<run-id>/`

Never default to OS user temp when repo-local `.tmp/` is sufficient.

## 4. Required Files Per Run

Every run bundle must contain:

- `RUN_MANIFEST.yaml`
- `inputs/PROMPT_INPUT.md`
- `inputs/SPEC_PACK_REF.md`
- `inputs/INPUT_HASHES.json`
- `inputs/RUN_ADDITIONAL_REQUIREMENTS.md`
- `inputs/CONTEXT_POLICY.yaml`
- `execution/SESSION_LOG.md`
- `execution/TOOL_LOG.jsonl`
- `outputs/CODEBASE_REF.yaml`
- `outputs/OUTPUT_HASHES.json`
- `validation/COMMANDS.md`
- `validation/RESULTS.md`
- `validation/SUMMARY.yaml`
- `handoff/HANDOFF.md`

## 5. Codebase Output Capture Rules

Each run must capture output codebase state by one of these methods:

1. In-tree output (preferred):
- code lands in `engines/<runtime>/<implementation-id>/`
- `outputs/CODEBASE_REF.yaml` records path + commit hash.

2. External repo output:
- `outputs/CODEBASE_REF.yaml` records remote URL + branch + commit hash.
- run must include an immutable snapshot artifact reference (for example bundle/tar hash) in `outputs/OUTPUT_HASHES.json`.

## 6. Prompt/Input Capture Rules

- Capture exact instruction text in `inputs/PROMPT_INPUT.md`.
- Record spec pack path and version (for example `docs/full-engine-spec/2026-02-27`) in `inputs/SPEC_PACK_REF.md`.
- Hash all normative inputs and record in `inputs/INPUT_HASHES.json`.
- Define managed-context policy in `inputs/CONTEXT_POLICY.yaml`:
  - allowed read paths,
  - forbidden read paths,
  - allowed write paths,
  - API-boundary exception protocol.
- `execution/TOOL_LOG.jsonl` must include structured path-access evidence per command (read/write paths + policy outcome).

## 7. Validation Capture Rules

Validation logs must include:
- commands executed,
- pass/fail status,
- environment assumptions,
- unresolved failures and blockers.

If tests cannot run, `validation/SUMMARY.yaml` must explicitly state why.
`validation/SUMMARY.yaml` must also include:
- `forbidden_access_count`,
- `api_boundary_exception_count`,
- `clean_room_attested`.

## 8. Immutability Rules

- A run bundle is immutable after close (`status: completed` or `status: blocked`).
- Corrections are appended as a new run, not by rewriting old run artifacts.
- Spec pack version is pinned per run and must not drift mid-run.

## 9. Multi-Implementation Naming Rules

`<implementation-id>` should include runtime and sequence:
- Rust: `coreengine-rs-01`, `coreengine-rs-02`, ...
- .NET: `coreengine-net-01`, `coreengine-net-02`, ...

## 10. Minimum Start Checklist

Before starting a new implementation run:
1. Pin spec pack version (`docs/full-engine-spec/<date>`).
2. Create implementation root under `engines/<runtime>/<implementation-id>/`.
3. Create run bundle from template under `runs/engine-impl/<run-id>/`.
4. Fill manifest and input files (including `inputs/CONTEXT_POLICY.yaml`) before code changes.

## 11. Templates

Use templates in:
- `runs/templates/engine_impl/`

Optional scaffold helper:
- `scripts/new_engine_run.ps1 -Runtime <rust|dotnet> -ImplementationId <id> -SpecPackVersion <date>`

