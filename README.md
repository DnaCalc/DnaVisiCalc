# DNA VisiCalc (DnaVisiCalc)

DNA VisiCalc is the Round 0 pathfinder for the larger DNA Calc program.

This repository now has explicit layer boundaries:
- `dnavisicalc-core`: pure calculation engine library crate.
- `dnavisicalc-file`: file read/write adapter crate.
- `dnavisicalc-tui`: modern terminal UI crate + `dnavisicalc` binary.

## Workspace layout

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

## Core scope
The `dnavisicalc-core` crate remains library-only and I/O free:
- VisiCalc-sized bounds (`A1`..`BK254`).
- Formula parser with arithmetic, comparisons, references, ranges.
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`, `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`, `CONCAT`, `LEN`.
- Dynamic arrays: `SEQUENCE`, `RANDARRAY`, spill references (`A1#`), `#SPILL`/`#REF` errors.
- Deterministic dependency graph and cycle detection.
- Manual/automatic recalc and epoch staleness tracking.

## File support
The `dnavisicalc-file` crate provides a deterministic text format and adapters:
- Save engine state to string/file.
- Load string/file to engine with validation and detailed parse errors.
- Format details: see `docs/FILE_FORMAT.md`.

## TUI
The `dnavisicalc-tui` crate provides a modern TUI using `ratatui` and `crossterm`:
- Grid navigation and cell editing.
- Command mode (`:w`, `:o`, `:mode`, `:recalc`, `:set`, `:q`).
- Inline keyboard help and full help popup (`?` or `F1`).
- Workbook header showing current file, save state, and recalc mode.
- Function list in the full help popup.
- Status messages that persist during navigation.
- Layered I/O abstraction so TUI behavior can be tested without real files or real terminals.

## Screenshots
Startup:

![Startup](docs/images/01_startup.png)

Editing text + formula concat:

![Editing](docs/images/02_editing.png)

Full help popup with supported function list:

![Help Popup](docs/images/03_help_popup.png)

Command mode:

![Command Mode](docs/images/04_command_mode.png)

## Windows release (v0.1)
Prebuilt Windows x64 artifacts are published on the GitHub release page:
- `https://github.com/DnaCalc/DnaVisiCalc/releases/tag/v0.1`

Quick start (Windows):
1. Download `dnavisicalc-v0.1-windows-x64.zip` from the release assets.
2. Extract the archive.
3. Run `dnavisicalc.exe` from the extracted folder.
4. Press `?` (or `F1`) in the app for full help, including the function list.

Run:

```bash
cargo run -p dnavisicalc-tui --bin dnavisicalc
```

Optional key-event tracing for terminal/input debugging:

```bash
DNAVISICALC_EVENT_TRACE=artifacts/windows/event-trace.log cargo run -p dnavisicalc-tui --bin dnavisicalc
```

```powershell
$env:DNAVISICALC_EVENT_TRACE = "artifacts/windows/event-trace.log"
cargo run -p dnavisicalc-tui --bin dnavisicalc
```

## Development

```bash
cargo test --workspace
```

Deep testing and hardening artifacts:
- `docs/testing/TESTING_PLAN.md`
- `docs/testing/TESTING_ROUNDS.md`
- `docs/testing/COVERAGE_SUMMARY.md`
- `docs/testing/WINDOWS_TERMINAL_KEY_REPRO.md`

Compatibility notes:
- `docs/VISICALC_COMPATIBILITY_MATRIX.md`

## Foundation relationship
This repository follows Foundation guidance with a lighter process suitable for pathfinding.
See:
- `docs/FOUNDATION_REQUIREMENTS_MAPPING.md`
- `../Foundation/CHARTER.md`
- `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
- `../Foundation/OPERATIONS.md`

## License
MIT (see `LICENSE`).
