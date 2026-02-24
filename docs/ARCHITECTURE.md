# Architecture (Pathfinder)

## 1. Layering

### Core (`dnavisicalc-core`)
- Purpose: deterministic spreadsheet calculation engine.
- No file/network/UI dependencies.
- Public API focused on:
  - cell addressing and bounds,
  - formula parse/eval,
  - dependency graph and calc order,
  - epoch and staleness state.

### File Adapter (`dnavisicalc-file`)
- Purpose: pure adapter between serialized workbook text and `dnavisicalc-core::Engine`.
- Reads/writes deterministic line format.
- Performs structural/semantic validation and reports line-specific parse errors.
- No TUI dependencies.

### TUI (`dnavisicalc-tui`)
- Purpose: interaction layer over core + file adapters.
- `App` state machine drives behavior; renderer is separate.
- Key mapping and command parsing are isolated and testable.
- `WorkbookIo` trait makes open/save pluggable (filesystem vs in-memory test doubles).

## 2. Dependency direction
- `dnavisicalc-core` <- no internal dependencies.
- `dnavisicalc-file` -> depends on `dnavisicalc-core`.
- `dnavisicalc-tui` -> depends on `dnavisicalc-core` and `dnavisicalc-file`.

No reverse dependency from core to adapters/UI.

## 3. Testability seams
- Core: deterministic pure logic, extensive unit/integration tests.
- File: parser/writer tests, broken-file corpus tests, fuzz/property tests.
- TUI:
  - `App` action reducer can be driven without terminal.
  - `WorkbookIo` trait allows in-memory fake storage.
  - Rendering verified with `ratatui::backend::TestBackend`.

## 4. Why this shape matches Foundation pathfinder intent
- Implementation-first with concrete artifact.
- Core boundary remains reusable for future adapters.
- Determinism and reproducibility remain explicit and test-gated.
- Process remains lightweight relative to Foundation umbrella project.