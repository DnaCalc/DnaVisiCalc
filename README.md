# DNA VisiCalc (DnaVisiCalc)

DNA VisiCalc is the Round 0 pathfinder for the larger DNA Calc program.

## Screenshots

### Getting Started

Empty grid on startup:

![Startup](docs/images/01_startup.png)

Editing a formula (`=A2*2`) with formatted header row:

![Editing](docs/images/02_editing.png)

Help popup with full function list and keybindings:

![Help Popup](docs/images/03_help_popup.png)

Command mode (`:mode manual`):

![Command Mode](docs/images/04_command_mode.png)

### Math & Science

8x8 multiplication table with rainbow column colors:

![Multiplication Table](docs/images/05_multiplication_table.png)

Scientific calculator (`SIN`, `COS`, `TAN`, `SQRT`, `EXP`, `LN`) with per-column colors:

![Scientific Calculator](docs/images/06_scientific_calculator.png)

### Financial

Financial model with `ROUND`, `PMT`, `NPV` across 5 periods:

![Financial Model](docs/images/07_financial_model.png)

Names-driven tax model (`TAX_RATE`, `DISCOUNT`) with formula references:

![Names Tax Model](docs/images/08_names_tax_model.png)

Loan calculator with named `RATE`/`TERM` and `PMT`, `PV`, `FV`, `NPV`:

![Loan Calculator](docs/images/09_loan_calculator.png)

### Formatting & Palette

Formatting showcase (bold, italic, bold+italic, decimals) with distinct fg/bg pairs:

![Formatting Showcase](docs/images/10_formatting_showcase.png)

Full 16-color palette as foreground, background, and bold number styling:

![Full Palette](docs/images/11_full_palette.png)

### Dynamic Arrays & Spill

`SEQUENCE(6,3)` with `SUM`/`AVG`/`MAX`/`MIN`/`COUNT` aggregate panel:

![Sequence Aggregates](docs/images/12_sequence_aggregates.png)

`RANDARRAY(6,3,0,100)` with aggregate statistics:

![RANDARRAY Lab](docs/images/13_randarray_lab.png)

`MAP` with lambda-driven `SEQUENCE` tiles (1x3 and 2x1 spill outputs):

![MAP Array Tiles](docs/images/14_map_array_tiles.png)

### Advanced Functions

`LET` + `LAMBDA` with named `BASE_RATE`/`RISK_ADJ` in a financial stress model:

![LET LAMBDA](docs/images/15_let_lambda.png)

`INDIRECT` (absolute + relative R1C1) and `OFFSET` behavior:

![INDIRECT R1C1 OFFSET](docs/images/16_indirect_r1c1_offset.png)

`LOOKUP` model with product table and order lookups:

![LOOKUP Model](docs/images/17_lookup_model.png)

### Logic & Text

Student gradebook with nested `IF`, `AND`, `OR`:

![Student Gradebook](docs/images/18_student_gradebook.png)

Text functions (`CONCAT`, `LEN`, `IF` on length):

![Text Functions](docs/images/19_text_functions.png)

### Clipboard

Paste Special picker (`Ctrl+V`, choose paste mode):

![Paste Special Picker](docs/images/20_paste_special_picker.png)

## Scope
The `dnavisicalc-core` crate stays library-only and I/O free:
- VisiCalc-sized bounds (`A1`..`BK254`).
- Formula parser with arithmetic, comparisons, references (including mixed/absolute `$` forms), ranges.
- Workbook names for reusable values/formulas (names can reference cells and other names).
- Per-cell formatting: decimals, text style (bold/italic), foreground/background palette colors.
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`, `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`, `CONCAT`, `LEN`, `LET`, `LAMBDA`, `MAP`, `INDIRECT`, `OFFSET`, `ROW`, `COLUMN`.
- `INDIRECT` supports both A1 and R1C1 reference text modes.
- `MAP` supports scalar and array-returning lambda outputs with deterministic tiling/broadcast behavior.
- Dynamic arrays: `SEQUENCE`, `RANDARRAY`, spill references (`A1#`), `#SPILL`/`#REF` errors.
- Structural row/column edits with deterministic formula/name rewrite (`insert/delete` row/col).
- Deterministic dependency graph, incremental dirty-closure recalculation, and cycle handling.
- Iterative SCC recalculation mode configuration (`enabled`, max iterations, tolerance).
- Volatility split for `Standard`, `Volatile`, and `ExternallyInvalidated` invalidation pathways.
- External UDF registration with volatility metadata.
- Engine-level controls/charts entities and opt-in change tracking journal APIs.
- Manual/automatic recalc with epoch staleness tracking.

## Files
The `dnavisicalc-file` crate provides deterministic workbook serialization:
- Current writer format: `DVISICALC v2` (reader supports `v1` + `v2`).
- Save engine state to string/file.
- Load string/file with validation and line-specific errors.
- Format details: `docs/FILE_FORMAT.md`.

## TUI
The `dnavisicalc-tui` crate provides a terminal UI using `ratatui` + `crossterm`:
- Grid navigation and cell editing (`Enter`/`e`/`F2`).
- Multi-cell selection with `Shift+Arrows` or `Shift+H/J/K/L`.
- System clipboard copy/paste (`Ctrl+C`, `Ctrl+V`) with Paste Special modes.
- `F9` force recalc shortcut (same behavior as `:r` / `:recalc`) in any mode.
- Command mode (`:w`, `:o`, `:mode`, `:recalc`, `:set`, `:q`).
- Name commands (`:name <NAME> <value|formula>`, `:name clear <NAME>`).
- Format commands (`:fmt decimals|bold|italic|fg|bg|clear ...`) and `Delete` to clear selected range contents.
- Structural commands (`:insrow`, `:delrow`, `:inscol`, `:delcol`, with aliases).
- Full help popup (`?` / `F1`) including the supported function list.
- Workbook header showing file, save status, and recalc mode.
- Status messages that persist during navigation.
- Layered `WorkbookIo` abstraction for deterministic tests without real filesystem/terminal dependencies.
- Tool-driving automation binaries:
  - `capture_scenes` (scene captures to txt/json/svg),
  - `capture_script` (keystroke script -> timeline + frame artifacts),
  - `capture_viewer` (CLI playback with transport controls).

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

Tool automation examples:

```bash
cargo run -p dnavisicalc-tui --bin capture_script -- scripts/tui/basic_edit.script artifacts/tui/basic 140 40
cargo run -p dnavisicalc-tui --bin capture_viewer -- artifacts/tui/basic/timeline.json
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
- `dnavisicalc-engine`: backend boundary/loader used by Rust adapters (`rust-core` and `dotnet-core` backends; configurable via `DNAVISICALC_COREENGINE`, pin DLL via `DNAVISICALC_COREENGINE_DLL`).
- `dnavisicalc-file`: serialization adapter.
- `dnavisicalc-tui`: interaction layer + `dnavisicalc` binary.

No reverse dependency from core to adapters or UI.

## Repository Layout

```text
crates/
  dnavisicalc-core/  # formulas, dependency graph, evaluation, epochs
  dnavisicalc-engine/ # engine backend boundary + config/loader
  dnavisicalc-coreengine-rust/ # in-workspace Rust C API backend DLL
  dnavisicalc-file/  # DVISICALC file format parser/writer
  dnavisicalc-tui/   # ratatui app, key mapping, command layer, binary
engines/
  rust/              # spec-derived Rust engine implementations (run outputs)
  dotnet/            # spec-derived .NET engine implementations
docs/
  ARCHITECTURE.md
  DYNAMIC_ARRAYS_DESIGN.md
  ENGINE_API.md
  ENGINE_API_RUST_APPENDIX.md
  ENGINE_REQUIREMENTS.md
  ENGINE_REQUIREMENTS_INTEGRATION_APPENDIX.md
  FILE_FORMAT.md
  OPERATIONS.md
  FOUNDATION_REQUIREMENTS_MAPPING.md
  SPEC_v0.md
  SPEC_v0_INTEGRATION_APPENDIX.md
  TUI_TESTABILITY.md
  full-engine-spec/
  testing/
runs/
  README.md
  engine-impl/
  templates/engine_impl/
```

Foundation relationship (pathfinder scope):
- `docs/FOUNDATION_REQUIREMENTS_MAPPING.md`
- `../Foundation/CHARTER.md`
- `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
- `../Foundation/OPERATIONS.md`

License: MIT (see `LICENSE`).
