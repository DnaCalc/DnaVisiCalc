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

Numerical model (formulas visible in input panel + financial functions):

![Numerical Model](docs/images/05_numerical_model.png)

Names-driven model (`TAX_RATE`, `DISCOUNT`) with formula references:

![Names Model](docs/images/06_names_model.png)

Paste Special picker (`Ctrl+V`, choose mode):

![Paste Special Picker](docs/images/07_paste_special_picker.png)

Paste Special applied (`Values+KeepDestFmt`):

![Paste Special Result](docs/images/08_paste_special_result.png)

Formatting pass (decimals, bold/italic, fg/bg palette commands on selection):

![Formatting and Colors](docs/images/09_formatting_colors.png)

Dynamic arrays (`SEQUENCE`, `RANDARRAY`, spill ranges, `A1#` aggregates):

![Dynamic Arrays](docs/images/10_dynamic_arrays.png)

Bioreactor operations dashboard (imaginary domain model, names + formulas + formatting):

![Bioreactor Dashboard](docs/images/11_bioreactor_dashboard.png)

Palette and text-format showcase (bold/italic/decimals with mixed color styling):

![Palette Showcase](docs/images/12_palette_showcase.png)

Names with `LET` + `LAMBDA` in a financial mini-model:

![Names LET LAMBDA](docs/images/13_names_let_lambda.png)

`INDIRECT` (A1 + R1C1 mode) and `OFFSET` behavior model:

![INDIRECT R1C1 OFFSET](docs/images/14_indirect_r1c1_offset.png)

`MAP` with dynamic-array lambda results (tiled spill outputs):

![MAP Array Tiles](docs/images/15_map_array_tiles.png)

Dynamic array lab view (multiple spills and aggregate summary panel):

![Dynamic Array Lab](docs/images/16_dynamic_array_lab.png)

## Scope
The `dnavisicalc-core` crate stays library-only and I/O free:
- VisiCalc-sized bounds (`A1`..`BK254`).
- Formula parser with arithmetic, comparisons, references, ranges.
- Workbook names for reusable values/formulas (names can reference cells and other names).
- Per-cell formatting: decimals, text style (bold/italic), foreground/background palette colors.
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`, `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`, `CONCAT`, `LEN`, `LET`, `LAMBDA`, `MAP`, `INDIRECT`, `OFFSET`, `ROW`, `COLUMN`.
- `INDIRECT` supports both A1 and R1C1 reference text modes.
- `MAP` supports scalar and array-returning lambda outputs with deterministic tiling/broadcast behavior.
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
- Grid navigation and cell editing (`Enter`/`e`/`F2`).
- Multi-cell selection with `Shift+Arrows` or `Shift+H/J/K/L`.
- System clipboard copy/paste (`Ctrl+C`, `Ctrl+V`) with Paste Special modes.
- Command mode (`:w`, `:o`, `:mode`, `:recalc`, `:set`, `:q`).
- Name commands (`:name <NAME> <value|formula>`, `:name clear <NAME>`).
- Format commands (`:fmt decimals|bold|italic|fg|bg|clear ...`) and `Delete` to clear selected range contents.
- Full help popup (`?` / `F1`) including the supported function list.
- Workbook header showing file, save status, and recalc mode.
- Status messages that persist during navigation.
- Layered `WorkbookIo` abstraction for deterministic tests without real filesystem/terminal dependencies.

## Windows Release (No Rust Needed)
Prebuilt Windows x64 artifacts are published on GitHub Releases:
- `https://github.com/DnaCalc/DnaVisiCalc/releases/tag/v0.2.0`

Quick start (Windows):
1. Download `dnavisicalc-v0.2.0-windows-x64.zip` from the release assets.
2. Extract the archive.
3. Run `dnavisicalc.exe` from the extracted folder.
4. Press `?` (or `F1`) in the app for full help and function coverage.

Detailed release/run docs:
- `docs/release/WINDOWS_RUN_v0.2.0.md`
- `docs/release/HELP_QUICK_REFERENCE_v0.2.0.md`

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
