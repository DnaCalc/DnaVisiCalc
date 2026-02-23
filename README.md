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