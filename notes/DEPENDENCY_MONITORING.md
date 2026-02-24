# DEPENDENCY_MONITORING.md

This note defines a lightweight dependency monitoring approach for DNA VisiCalc.

## 1. Why monitor dependencies

- Keep the core engine small and stable.
- Control supply-chain and security risk.
- Prevent transitive dependency drift from silently increasing build/runtime complexity.
- Preserve deterministic, testable behavior across layers.

## 2. Current baseline (2026-02-24)

Workspace shape:
- `dnavisicalc-core`: 0 direct runtime deps.
- `dnavisicalc-file`: 2 direct runtime deps (`dnavisicalc-core`, `thiserror`).
- `dnavisicalc-tui`: 5 direct runtime deps (`anyhow`, `crossterm`, `dnavisicalc-core`, `dnavisicalc-file`, `ratatui`).

Workspace totals:
- Direct runtime external deps: 4 (`anyhow`, `crossterm`, `ratatui`, `thiserror`).
- Direct dev deps: 3 (`assert_cmd`, `proptest`, `tempfile`).
- Unique runtime crates (normal): 57 total (54 external + 3 workspace crates).
- Unique crates (all kinds): 92.
- Runtime max depth: 5.

Known duplicate/version splits:
- `crossterm` 0.28.1 and 0.29.0.
- `unicode-width` 0.1.14 and 0.2.0.
- `getrandom` 0.3.x and 0.4.x (dev graph).

## 3. Standard monitoring commands

Note: in some shells, Cargo is not on `PATH`; use:
- Windows: `"$env:USERPROFILE\\.cargo\\bin\\cargo.exe"`

Core inventory:
- `cargo metadata --format-version 1 --no-deps`
- `cargo tree --workspace -e normal`
- `cargo tree --workspace --duplicates`

Risk/security:
- `cargo audit`
- `cargo deny check advisories licenses bans sources`
- `cargo geiger`

Maintenance/drift:
- `cargo outdated -R`
- `cargo udeps --all-targets`

Build/cost:
- `cargo build --timings`
- `cargo bloat --release --crates`

## 4. Monitoring cadence

- On every PR:
  - `cargo tree --workspace --duplicates`
  - `cargo audit`
  - `cargo test --workspace`
- Weekly:
  - `cargo outdated -R`
  - `cargo udeps --all-targets`
  - `cargo geiger`
- Monthly:
  - `cargo deny check advisories licenses bans sources`
  - `cargo build --timings`
  - `cargo bloat --release --crates`

## 5. Guardrails and triggers

Investigate and document if any occur:
- Runtime unique crate count increases by >10% without clear feature justification.
- New duplicate major/minor versions appear on runtime path.
- Any high/critical advisory appears.
- Any new external runtime dependency is added to `dnavisicalc-core`.
- Build time regresses significantly (team-defined threshold; suggest 20%+).

## 6. Layer-specific policy

- `dnavisicalc-core`: keep runtime dependencies at zero unless a strict technical case exists.
- `dnavisicalc-file`: prefer tiny, well-audited utility crates only.
- `dnavisicalc-tui`: TUI stack can carry most transitive weight; keep it isolated from core.

## 7. Practical next cleanup targets

1. Attempt to eliminate the `crossterm` version split by aligning direct/indirect versions.
2. Re-check whether `unicode-width` duplication can be reduced via dependency alignment.
3. Add CI steps for `cargo audit` and `cargo tree --duplicates` to fail fast on regressions.
