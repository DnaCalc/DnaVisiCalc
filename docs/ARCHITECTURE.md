# Architecture (Pathfinder)

## 1. Layering

### Core (`dnavisicalc-core`)
- Purpose: deterministic spreadsheet engine with no file/network/UI dependencies.
- Public API includes:
  - cell/name input and query,
  - formula parse/eval and dependency-ordered recalculation,
  - epoch/staleness model and auto/manual recalc modes,
  - incremental invalidation (`invalidate_volatile`, `invalidate_udf`, stream ticks),
  - structural row/column insert/delete rewrites,
  - iterative SCC calculation configuration,
  - controls/charts/change-journal engine entities,
  - formatting and deterministic bulk enumeration.

### Engine Boundary (`dnavisicalc-engine`)
- Purpose: backend boundary/loader layer used by Rust adapters.
- Provides an `Engine` wrapper around the active core backend plus backend metadata (`coreengine()`).
- Current backend set:
  - `rust-core` (default).
- Backend selection mechanism:
  - `DNAVISICALC_COREENGINE` (`rust-core`, aliases: `rust`, `core`).
- Keeps file/TUI crates decoupled from direct `dnavisicalc-core` construction, making C-API-backed engines pluggable behind one seam.

### File Adapter (`dnavisicalc-file`)
- Purpose: deterministic serialization adapter for engine state.
- Current persisted scope (`DVISICALC v2`):
  - recalc mode,
  - iteration config,
  - dynamic-array strategy,
  - cell inputs,
  - name inputs,
  - control definitions,
  - chart definitions,
  - cell formats.
- Backward read compatibility: `DVISICALC v1` is accepted.
- Provides strict line-level validation and deterministic load/apply behavior.
- No TUI dependencies.

### TUI (`dnavisicalc-tui`)
- Purpose: interaction layer over core + file adapter.
- `App` reducer drives behavior, renderer is separate.
- Command parser and key mapping are explicit/testable.
- `WorkbookIo` keeps persistence pluggable (filesystem and in-memory doubles).
- Automation seams are explicit in [TUI_TESTABILITY.md](TUI_TESTABILITY.md).

## 2. Dependency Direction
- `dnavisicalc-core` <- no reverse dependency.
- `dnavisicalc-engine` -> depends on `dnavisicalc-core`.
- `dnavisicalc-file` -> depends on `dnavisicalc-engine`.
- `dnavisicalc-tui` -> depends on `dnavisicalc-engine` and `dnavisicalc-file`.

Core remains reusable for future C API boundary work and alternate host adapters.

## 3. Testability and Automation Seams
- Core:
  - deterministic logic, property/integration coverage,
  - reproducible recalculation and mutation behavior.
- File:
  - parser/writer corpus and round-trip coverage,
  - explicit parse diagnostics and version-gating tests.
- TUI:
  - action-level and key-level scripting without real terminal loops,
  - render capture with `ratatui::backend::TestBackend`,
  - deterministic viewport sizing for scripted runs,
  - artifact pipeline for txt/json/svg frame snapshots and timeline playback.

## 4. Implemented UI Automation Surface
- Stable capture protocol with full-color frame buffers and cursor metadata (`capture` module).
- Deterministic keystroke-driven script capture (`capture_script` binary).
- Frame-to-image export via SVG snapshots.
- CLI timeline viewer (`capture_viewer`) with play/pause, +/-1, +/-15, and speed controls.

Details are specified in [TUI_TESTABILITY.md](TUI_TESTABILITY.md) and tracked in the testing plan.

## 5. Why this shape fits Pathfinder intent
- Keeps implementation-first momentum with strict seams.
- Preserves deterministic behavior as a default quality bar.
- Supports incremental hardening without collapsing crate boundaries.
