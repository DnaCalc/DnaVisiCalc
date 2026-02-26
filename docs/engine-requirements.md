# Engine Requirements — DNA VisiCalc

Formalized requirements for the `dnavisicalc-core` engine, scoped to VisiCalc (Round 0). Drawn from Foundation documents ([CHARTER.md](../../Foundation/CHARTER.md), [ARCHITECTURE_AND_REQUIREMENTS.md](../../Foundation/ARCHITECTURE_AND_REQUIREMENTS.md)) and the current Rust implementation.

## 1. Scope

This document covers the DNA VisiCalc pathfinder engine only. It does not attempt to specify the full Foundation protocol surface (multi-sheet workbooks, OpLog, collaboration, XLL, VBA, structural rewrites). Requirements here must be compatible with evolving toward the Foundation architecture but are not constrained by it.

The engine is a single-sheet calculation engine with:
- Cell and named-value storage
- Formula parsing and evaluation
- Dependency-ordered recalculation
- Dynamic array spill semantics
- Cell formatting metadata
- Volatile function and stream support
- Externally driven execution (no internal threads or timers)

## 2. Execution Model (REQ-EXEC)

### REQ-EXEC-001: Externally Driven

The engine performs no internal threading, timer management, or autonomous I/O. All computation occurs synchronously within caller-initiated function calls. The caller is responsible for scheduling recalculation, volatile refreshes, and stream ticks.

*Rationale:* Foundation CONSTR-002 ("File and network I/O are adapters outside core"). Externally driven execution enables deterministic testing and embedding in any host environment.

### REQ-EXEC-002: State Machine

The engine operates in two observable states:

```
STABLE ──mutation──▶ DIRTY ──recalculate──▶ STABLE
```

- **STABLE:** `stabilized_epoch == committed_epoch`. All derived values are current.
- **DIRTY:** `stabilized_epoch < committed_epoch`. At least one mutation has occurred since the last recalculation.

In Automatic recalc mode, the engine transitions back to STABLE immediately after each mutation (recalculation is triggered internally by the mutation call). In Manual mode, the engine remains DIRTY until the caller explicitly requests recalculation.

### REQ-EXEC-003: Epoch Tracking

The engine maintains two monotonically increasing epoch counters:

- **`committed_epoch`** — incremented on every mutation (cell set, cell clear, name set, name clear, format change, stream tick, clear-all).
- **`stabilized_epoch`** — set to `committed_epoch` after successful recalculation.

Derived cell values carry a `value_epoch` indicating which `committed_epoch` they were computed against. A value is **stale** when `value_epoch < committed_epoch`.

*Foundation reference:* ARCHITECTURE_AND_REQUIREMENTS §3.3 (Epoch Model).

### REQ-EXEC-004: Recalculation Modes

Two modes:

| Mode | Behavior |
|------|----------|
| **Automatic** | Every mutation triggers immediate recalculation. The engine is always STABLE after a successful mutation. |
| **Manual** | Mutations do not trigger recalculation. The engine remains DIRTY until the caller explicitly calls `recalculate()`. |

The mode is queryable and settable at any time. Switching from Manual to Automatic does not automatically trigger recalculation — the caller is responsible for requesting it.

## 3. Cell Operations (REQ-CELL)

### REQ-CELL-001: Addressing

Cells are addressed by `(col, row)` pairs where both are 1-based `u16` values. Column 1 corresponds to "A", column 2 to "B", etc.

Sheet bounds define the maximum column and row. The default bounds are 63 columns × 254 rows. Custom bounds may be specified at engine creation.

### REQ-CELL-002: A1 String Addressing

The engine provides A1-style string addressing as a convenience layer over numeric `(col, row)` addressing. A1 references are parsed and validated against the current sheet bounds.

### REQ-CELL-003: Input Types

A cell may contain one of:

| Input Type | Description |
|-----------|-------------|
| **Number** | IEEE 754 `f64` literal |
| **Text** | UTF-8 string |
| **Formula** | Source string beginning with `=` or `@`, parsed into an AST |
| **Empty** | No input (cell cleared) |

### REQ-CELL-004: Typed Setters

Cell inputs are set through typed functions (`set_number`, `set_text`, `set_formula`) rather than a single polymorphic setter. A separate `clear_cell` function removes all input. A combined `set_cell_input` accepting a tagged union is provided as a convenience.

### REQ-CELL-005: Mutation Advances Epoch

Every mutation to cell content increments `committed_epoch` and (in Automatic mode) triggers recalculation.

### REQ-CELL-006: Bounds Checking

All cell operations validate that the target address is within the engine's sheet bounds. Operations on out-of-bounds cells return an error without modifying state.

### REQ-CELL-007: Input Query

The engine supports querying a cell's current input (returning the input type and original source text for formulas, the number for numeric inputs, and the text for text inputs). Querying an empty cell returns `None`/empty.

### REQ-CELL-008: Spill Detection Before Mutation

Before allowing a cell to be edited, the caller can query whether a cell is a spill member (owned by another cell's dynamic array expansion). The engine provides `spill_anchor_for_cell` to detect this condition.

## 4. Named Values (REQ-NAME)

### REQ-NAME-001: Same Input Types as Cells

Named values support the same input types as cells: Number, Text, and Formula. Named formulas can reference cells and other names.

### REQ-NAME-002: Name Validation

Name identifiers must satisfy:
- Non-empty after trimming
- First character is ASCII alphabetic or underscore
- Remaining characters are ASCII alphanumeric or underscore
- Stored as uppercase (case-insensitive matching)
- Must not conflict with: boolean literals (`TRUE`, `FALSE`), cell references (e.g. `A1`, `BK254`), or built-in function names

Invalid names produce an error.

### REQ-NAME-003: Typed Setters

Named values use the same typed-setter pattern as cells: `set_name_number`, `set_name_text`, `set_name_formula`, `clear_name`.

### REQ-NAME-004: Name Query

The engine supports querying a name's current input type and value. Querying a non-existent name returns `None`/empty.

## 5. Recalculation (REQ-CALC)

### REQ-CALC-001: Deterministic Topological Order

Recalculation evaluates formulas in dependency order determined by topological sort of the dependency graph. Literal cells (numbers, text) are resolved first, then formula cells in topological order, then named formulas in sorted name order.

*Foundation reference:* ARCHITECTURE_AND_REQUIREMENTS §3.4 ("Deterministic mode exists").

### REQ-CALC-002: Dependency Graph Construction

The engine builds a calculation tree (dependency graph) from all formula cells. The graph detects circular dependencies and reports them as `CellError::Cycle` errors on affected cells.

### REQ-CALC-003: Automatic vs Manual Modes

In Automatic mode, `recalculate()` is called internally after every mutation. In Manual mode, the caller must call `recalculate()` explicitly. The `recalculate()` function is always available regardless of mode.

### REQ-CALC-004: Stream Tick

The engine supports stream cells (cells containing `STREAM(topic, period)` formulas). Stream state is managed through:

- **`tick_streams(elapsed_secs)`** — accumulates elapsed time for each stream cell. When accumulated time reaches the period, the counter advances and `committed_epoch` increments. Returns `true` if any counter advanced.
- **`has_stream_cells()`** — returns whether any stream cells exist.

The caller is responsible for calling `tick_streams` at appropriate intervals and triggering recalculation when it returns `true`.

### REQ-CALC-005: Volatile Recalculation

Cells containing volatile functions (`NOW`, `RAND`, `RANDARRAY`, `STREAM`) must be re-evaluated on every recalculation. The engine provides `has_volatile_cells()` so the caller can determine whether periodic recalculation is needed.

### REQ-CALC-006: Value Types

Evaluation produces values of type:

| Value Type | Description |
|-----------|-------------|
| **Number** | IEEE 754 `f64` |
| **Text** | UTF-8 string |
| **Bool** | `true` / `false` |
| **Blank** | Empty cell value |
| **Error** | Cell error with error kind |

### REQ-CALC-007: Cell State

Each cell's computed state consists of:
- **value** — the evaluated Value
- **value_epoch** — the `committed_epoch` at which this value was computed
- **stale** — boolean flag: `true` when `value_epoch < committed_epoch`

## 6. Dynamic Arrays and Spill (REQ-SPILL)

### REQ-SPILL-001: Spill Semantics

Formula cells that produce array results expand ("spill") into adjacent cells. The formula cell is the **anchor**; expanded cells are **members**. Members are not directly editable.

### REQ-SPILL-002: Spill Blocking

A spill is blocked when a member cell would overlap with an existing input cell or another spill region. Blocked spills produce a `CellError::Spill` on the anchor cell.

### REQ-SPILL-003: Spill Queries

The engine provides:
- **spill_anchor_for_cell(cell)** — given a spill member, returns the anchor cell
- **spill_range_for_cell(cell)** — given any cell in a spill region, returns the full range
- **spill_range_for_anchor(cell)** — given an anchor, returns its spill range

## 7. Formatting (REQ-FMT)

### REQ-FMT-001: Metadata-Only

Cell formatting is pure metadata — it does not affect calculation. Format changes increment `committed_epoch` and set `stabilized_epoch` equal to it (no recalculation needed).

### REQ-FMT-002: Format Properties

Each cell's format contains:

| Property | Type | Default |
|----------|------|---------|
| **decimals** | `Option<u8>` (0–9) | None (auto) |
| **bold** | boolean | false |
| **italic** | boolean | false |
| **fg** | `Option<PaletteColor>` | None |
| **bg** | `Option<PaletteColor>` | None |

### REQ-FMT-003: 16-Color Palette

Colors are drawn from a fixed 16-color named palette: Mist, Sage, Fern, Moss, Olive, Seafoam, Lagoon, Teal, Sky, Cloud, Sand, Clay, Peach, Rose, Lavender, Slate.

Colors are identified by name strings (case-insensitive) and by enumeration index.

### REQ-FMT-004: Default Format Optimization

A cell with all-default formatting has no format entry stored. Setting a format to all-defaults removes the entry.

## 8. Error Model (REQ-ERR)

### REQ-ERR-001: Engine Errors

Engine-level operations may fail with structured errors:

| Error Kind | Description |
|-----------|-------------|
| **Address** | Invalid cell reference string |
| **Parse** | Formula parse failure |
| **Dependency** | Dependency graph construction failure |
| **Name** | Invalid name identifier |
| **OutOfBounds** | Cell address outside sheet bounds |

Engine errors carry a human-readable Display representation.

### REQ-ERR-002: Cell Value Errors

Cell evaluation may produce error values:

| Error Kind | Description |
|-----------|-------------|
| **DivisionByZero** | Division by zero |
| **Value** | Type mismatch or invalid argument |
| **Name** | Unknown function name |
| **UnknownName** | Unknown named value |
| **Ref** | Invalid cell reference |
| **Spill** | Dynamic array spill failure |
| **Cycle** | Circular dependency detected |

Cell errors carry descriptive messages and (for Cycle) the cycle path.

### REQ-ERR-003: Mappable to Integer Codes

Both engine errors and cell value errors must be mappable to distinct integer status codes for the C API boundary. The mapping must be stable across versions.

## 9. Bulk Enumeration (REQ-BULK)

### REQ-BULK-001: All Cell Inputs

The engine provides enumeration of all non-empty cell inputs as `(CellRef, CellInput)` pairs in deterministic order (sorted by cell address: column-major, then row).

### REQ-BULK-002: All Name Inputs

The engine provides enumeration of all named values as `(String, NameInput)` pairs in deterministic order (sorted alphabetically by name).

### REQ-BULK-003: All Cell Formats

The engine provides enumeration of all non-default cell formats as `(CellRef, CellFormat)` pairs in deterministic order (sorted by cell address).

### REQ-BULK-004: Serialization Round-Trip

The three bulk enumeration functions together with the corresponding setter functions are sufficient to serialize and deserialize the complete engine state (modulo computed values, which are regenerated by recalculation).

*Validation:* The `dnavisicalc-file` crate demonstrates this by using `all_cell_inputs()`, `all_name_inputs()`, and `all_cell_formats()` for serialization and `set_cell_input()`, `set_name_input()`, `set_cell_format()`, and `recalculate()` for deserialization.

## 10. Non-Goals

The following are explicitly out of scope for the VisiCalc engine:

- **Multi-sheet workbooks** — single sheet only
- **Row/column insert/delete** — structural rewrites are deferred to later rounds
- **Collaboration** — no OpLog, no replication
- **XLL/UDF** — no external function registration
- **VBA/macros** — no scripting runtime
- **File format specifics** — file I/O is an external adapter, not an engine concern
- **OOXML/Excel fidelity** — not a goal for this round
- **Internal threading** — the engine is purely synchronous and externally driven
- **Undo/redo** — the caller is responsible for managing undo state if desired
