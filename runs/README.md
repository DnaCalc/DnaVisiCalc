# Engine Implementation Runs

This directory contains immutable run bundles for core engine implementation work.
Runs use managed-context policy input (`inputs/CONTEXT_POLICY.yaml`) to isolate implementation context from existing code.

Create each run by copying `runs/templates/engine_impl/` to:
- `runs/engine-impl/<run-id>/`

Example run id:
- `20260228-091500Z_dotnet_coreengine-net-01_2026-02-27`

Reference:
- `docs/OPERATIONS.md`
- `scripts/new_engine_run.ps1`
