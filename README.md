# DNA VisiCalc (DnaVisiCalc)

DNA VisiCalc is the Round 0 pathfinder for the larger DNA Calc program.
It is a deliberately small, Rust-based spreadsheet calculation core that exists to make ideas concrete quickly.

## Intent
- Build a tiny but real calculation engine, first.
- Use it to explore semantics, dependency handling, and deterministic recalculation.
- Keep architecture clean and extensible without importing full project complexity.

## Current scope
- Core engine library only (no UI, no file I/O).
- Formula parsing and evaluation over a bounded sheet.
- Basic function set and cell/range references.
- Deterministic recalculation with explicit tests.

See `docs/SPEC_v0.md` for the normative initial scope and implementation plan.

## Implemented in v0.1.0
- VisiCalc-sized bounds: `A1` through `BK254`.
- Formula parser with arithmetic, comparison operators, `A1` refs, and ranges (`A1...B7`, `A1:B7`).
- Function support: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT` (`@` prefix accepted).
- Dependency graph + deterministic calc ordering + cycle detection.
- Manual and automatic recalc modes.
- Epoch tracking: `committed_epoch`, `stabilized_epoch`, and per-cell staleness.
- Integration tests covering parser/evaluator/engine behaviors.

## Development
Prerequisite: Rust toolchain (stable) with `cargo`.

```bash
cargo test
```

## Out of scope (for now)
- Full Excel compatibility.
- OOXML import/export.
- Collaboration or networking.
- Macro/VBA runtime.
- Formal proof toolchain integration.

## Foundation relationship
The full DNA Calc mission/doctrine lives in `../Foundation`.
This repository applies that direction in a lighter, implementation-first way suitable for a pathfinder.

## License
MIT (see `LICENSE`).
