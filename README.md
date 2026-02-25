# DNA VisiCalc (DnaVisiCalc)

DNA VisiCalc is the Round 0 pathfinder for the larger DNA Calc program.

## Screenshots
Startup:

![Startup](docs/images/01_startup.png)

Editing text + concat formula:

![Editing](docs/images/02_editing.png)

Help popup with function list:

![Help Popup](docs/images/03_help_popup.png)

Command mode:

![Command Mode](docs/images/04_command_mode.png)

Numerical model (formulas + financial functions):

![Numerical Model](docs/images/05_numerical_model.png)

## Scope
The `dnavisicalc-core` crate stays library-only and I/O free:
- VisiCalc-sized bounds (`A1`..`BK254`).
- Formula parser with arithmetic, comparisons, references, ranges.
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`, `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`, `CONCAT`, `LEN`.
- Dynamic arrays: `SEQUENCE`, `RANDARRAY`, spill references (`A1#`), `#SPILL`/`#REF` errors.
- Deterministic dependency graph and cycle detection.
- Manual/automatic recalc with epoch staleness tracking.

## Files
The `dnavisicalc-file` crate provides deterministic workbook serialization:
- Save engine state to string/file.
- Load string/file with validation and line-specific errors.
- Format details: `docs/FILE_FORMAT.md`.

## TUI
The `dnavisicalc-tui` crate provides a terminal UI using `ratatui` + `crossterm`:
- Grid navigation and cell editing.
- Command mode (`:w`, `:o`, `:mode`, `:recalc`, `:set`, `:q`).
- Full help popup (`?` / `F1`) including the supported function list.
- Workbook header showing file, save status, and recalc mode.
- Status messages that persist during navigation.
- Layered `WorkbookIo` abstraction for deterministic tests without real filesystem/terminal dependencies.

## Windows Release (No Rust Needed)
Prebuilt Windows x64 artifacts are published on GitHub Releases:
- `https://github.com/DnaCalc/DnaVisiCalc/releases/tag/v0.1.2`

Quick start (Windows):
1. Download `dnavisicalc-v0.1.2-windows-x64.zip` from the release assets.
2. Extract the archive.
3. Run `dnavisicalc.exe` from the extracted folder.
4. Press `?` (or `F1`) in the app for full help and function coverage.

Detailed release/run docs:
- `docs/release/WINDOWS_RUN_v0.1.2.md`
- `docs/release/HELP_QUICK_REFERENCE_v0.1.2.md`

## Development
Run locally with Cargo:

```bash
cargo run -p dnavisicalc-tui --bin dnavisicalc
```

Run all tests:

```bash
cargo test --workspace
```

Optional key-event tracing for terminal/input debugging:

```powershell
$env:DNAVISICALC_EVENT_TRACE = "artifacts/windows/event-trace.log"
cargo run -p dnavisicalc-tui --bin dnavisicalc
```

Deep testing and hardening artifacts:
- `docs/testing/TESTING_PLAN.md`
- `docs/testing/TESTING_ROUNDS.md`
- `docs/testing/COVERAGE_SUMMARY.md`
- `docs/testing/WINDOWS_TERMINAL_KEY_REPRO.md`

Compatibility notes:
- `docs/VISICALC_COMPATIBILITY_MATRIX.md`

## Layers Story
This repository keeps explicit boundaries:
- `dnavisicalc-core`: pure calculation engine.
- `dnavisicalc-file`: serialization adapter.
- `dnavisicalc-tui`: interaction layer + `dnavisicalc` binary.

No reverse dependency from core to adapters or UI.

## Repository Layout

```text
crates/
  dnavisicalc-core/  # formulas, dependency graph, evaluation, epochs
  dnavisicalc-file/  # DVISICALC file format parser/writer
  dnavisicalc-tui/   # ratatui app, key mapping, command layer, binary
docs/
  ARCHITECTURE.md
  DYNAMIC_ARRAYS_DESIGN.md
  FILE_FORMAT.md
  FOUNDATION_REQUIREMENTS_MAPPING.md
  SPEC_v0.md
  TUI_TESTABILITY.md
  testing/
```

Foundation relationship (pathfinder scope):
- `docs/FOUNDATION_REQUIREMENTS_MAPPING.md`
- `../Foundation/CHARTER.md`
- `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
- `../Foundation/OPERATIONS.md`

License: MIT (see `LICENSE`).
