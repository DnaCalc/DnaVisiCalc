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
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`.
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
- Layered I/O abstraction so TUI behavior can be tested without real files or real terminals.

Run:

```bash
cargo run -p dnavisicalc-tui --bin dnavisicalc
```

## Development

```bash
cargo test --workspace
```

## Foundation relationship
This repository follows Foundation guidance with a lighter process suitable for pathfinding.
See:
- `docs/FOUNDATION_REQUIREMENTS_MAPPING.md`
- `../Foundation/CHARTER.md`
- `../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md`
- `../Foundation/OPERATIONS.md`

## License
MIT (see `LICENSE`).