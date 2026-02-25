# DNA VisiCalc Spec v0 (Initial Pathfinder Scope)

## 1. Purpose
This document defines the first executable slice of DnaVisiCalc: a Rust core calculation engine library with no UI and no I/O adapters.

The goal is to create a working, testable seed that captures part of the intended DNA Calc direction:
- deterministic recalculation,
- explicit dependency/calc-tree handling,
- bounded and reproducible semantics.

## 2. VisiCalc-inspired scope anchor
Historical VisiCalc references indicate:
- A classic sheet shape of 63 columns by 254 rows (A..BK, 1..254) in the user model.
- A1-style references and range notation using `...` (for example `@SUM(B2 ... M2)`).
- Function syntax prefixed with `@` (for example `@SUM(...)`).
- Manual and automatic recalculation modes.
- Recalculation order as a visible concept.

DnaVisiCalc v0 intentionally mirrors this style, while implementing only a tiny subset.

## 3. Normative v0 requirements

### 3.1 Engine boundary
- Library-only crate.
- No file format readers/writers.
- No terminal, GUI, network, or async runtime dependency.
- Pure in-memory API.

### 3.2 Sheet model
- Max dimensions: 63 columns x 254 rows.
- Address format: A1 notation.
- Cell contents:
  - numeric literal,
  - text literal,
  - formula expression,
  - blank.

### 3.3 Formula language v0
Supported syntax:
- Arithmetic: `+ - * / ^`.
- Text literals: `"..."` (with `""` escape for quotes).
- Concatenation: `&`.
- Parentheses and unary `+ -`.
- Comparisons: `= <> < <= > >=`.
- Cell references: `A1`, `BK254`.
- Workbook names: `TAX_RATE`, `_FOO` (named values/formulas).
- Ranges: `A1...B7` and `A1:B7` (alias accepted for convenience).
- Functions (case-insensitive; `@NAME` and `NAME` accepted):
  - `SUM`
  - `MIN`
  - `MAX`
  - `AVERAGE`
  - `COUNT`
  - `IF`
  - `AND`
  - `OR`
  - `NOT`
  - `ABS`
  - `INT`
  - `ROUND`
  - `SIGN`
  - `SQRT`
  - `EXP`
  - `LN`
  - `LOG10`
  - `SIN`
  - `COS`
  - `TAN`
  - `ATN`
  - `PI`
  - `NPV`
  - `PV`
  - `FV`
  - `PMT`
  - `LOOKUP`
  - `NA`
  - `ERROR`
  - `CONCAT`
  - `LEN`
  - `LET`
  - `LAMBDA`
  - `MAP`
  - `INDIRECT`
  - `OFFSET`
  - `ROW`
  - `COLUMN`

Explicit exclusions for v0:
- Date/time semantics.
- Volatile functions.
- Iterative/circular calculation semantics beyond deterministic cycle detection error.

### 3.3.2 Implemented formula extensions
- Lexical bindings and lambdas:
  - `LET(name, value, ..., result)` evaluates pairs left-to-right with local lexical scope.
  - `LAMBDA(param..., body)` returns a closure value.
  - `MAP(array..., LAMBDA(...))` performs element-wise lambda application with scalar broadcasting.
  - `MAP` lambda results may be scalar or array; array results are deterministically tiled/broadcast into the final spill.
- Reference indirection helpers:
  - `INDIRECT(text_ref[, a1])` supports both A1 and R1C1 text references (single cell, range, or spill reference with `#`).
  - `OFFSET(reference, rows, cols[, height, width])` returns reference-backed values/ranges.
  - `ROW([reference])` / `COLUMN([reference])` return row/column indices.

### 3.3.1 Workbook names (implemented extension)
- Workbook-level names can be assigned to numeric/text values or formulas.
- Name formulas can reference cells and other names.
- Names are case-insensitive and normalized to uppercase.
- Invalid names are rejected when they:
  - conflict with cell addresses (for example `A1`),
  - conflict with built-in function names,
  - violate identifier rules (must start with letter or `_`; then letters/digits/`_`).

### 3.4 Calc tree and evaluation
- Parse formulas into an expression AST.
- Build a dependency graph over formula cells.
- Build a deterministic calc order from graph topology.
- Detect circular references and return explicit errors.
- Evaluate deterministically from immutable input state per recalc pass.

### 3.5 Epoch/staleness seed
To keep continuity with the broader architecture, engine state tracks:
- `committed_epoch` (increments on edits),
- `stabilized_epoch` (advances on successful recalc),
- `value_epoch` per computed cell.

A computed value is stale when `value_epoch < committed_epoch`.

### 3.6 Recalc modes
- `Automatic`: edits trigger immediate full recalc.
- `Manual`: edits update `committed_epoch` and defer recalc until explicitly requested.

### 3.7 Cell formatting (implemented extension)
- Per-cell formatting metadata is supported and persisted:
  - decimals for numeric display (`0..9` or unset),
  - text style flags (`bold`, `italic`),
  - foreground/background palette colors (16-name nature-soft palette).
- Formatting does not change formula evaluation semantics.

## 4. Initial crate and module layout

```text
src/
  lib.rs
  address.rs      # A1 refs, bounds, range expansion
  ast.rs          # expression/calc node definitions
  parser.rs       # formula lexer + parser
  deps.rs         # dependency extraction and graph/calc order
  eval.rs         # expression evaluator and function semantics
  engine.rs       # engine state, epochs, recalc API
tests/
  parser_tests.rs
  eval_tests.rs
  engine_tests.rs
```

## 5. Testing strategy and infrastructure

### 5.1 Test structure
- Unit tests per module.
- Integration tests for end-to-end recalc flows.
- Fixed regression tests for any discovered bug.

### 5.2 Required test coverage areas
- Address parsing and bounds enforcement.
- Formula parsing precedence and associativity.
- Range parsing and expansion.
- Function arity/type behavior.
- Dependency closure and deterministic evaluation order.
- Cycle detection.
- Manual vs automatic recalc behavior.
- Epoch/stale flag semantics.

### 5.3 Near-term extension hooks
- Property tests (`proptest`) for parser/evaluator invariants.
- Corpus-based formula fuzzing.
- Snapshot-based deterministic trace tests.

## 6. Minimal acceptance criteria for v0
- `cargo test` passes on a clean checkout.
- Example workbook-like scenarios evaluate correctly.
- Calc order is deterministic for identical inputs.
- Cycle, parse, and bounds failures are explicit and test-covered.

## 7. Non-goals
- Full VisiCalc compatibility.
- Excel compatibility work.
- UI and editing model.
- Storage, import/export, and adapters.
- Concurrency scheduling.

## 8. References
- VisiCalc 1979 manual scan (sheet limits, `@` functions, recalculation controls):
  - https://archive.org/details/Visicalc_1979_Software_Arts
  - OCR text file: https://archive.org/download/Visicalc_1979_Software_Arts/Visicalc_1979_Software_Arts_djvu.txt
- Bob Frankston, "Implementing VisiCalc" (historical implementation notes and original constraints):
  - https://www.landley.net/history/mirror/apple2/implementingvisicalc.html
- Dan Bricklin, "Why VisiCalc was important" (early notation examples such as `A1`, `SUM(A1..A7)`):
  - http://www.bricklin.com/firstspreadsheetquestion.htm
