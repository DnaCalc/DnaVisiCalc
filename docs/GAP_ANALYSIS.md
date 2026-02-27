# Gap Analysis: DNA VisiCalc as Pathfinder

## Context

DNA VisiCalc exists to put the DNA Calc project "in harm's way" for the hard
engine problems before committing to the full Foundation architecture. The C API
specification ([ENGINE_API.md](ENGINE_API.md)) defines a clean boundary layer. This document
asks: **does the current scope exercise enough of the fundamental spreadsheet
engine challenges to be a useful pathfinder?**

It identifies what we cover well, what we're missing from the Excel engine
domain, and how the gaps relate to the Foundation's stated Round 0 scope.

## What DNA VisiCalc Already Exercises Well

Real engine problems being solved today:

- **Dependency-ordered recalculation** — topological sort, cycle detection, eval pipeline
- **Dynamic arrays with spill semantics** — three pluggable strategies, blocking detection, spill anchor/member model
- **Epoch-based staleness** — committed_epoch / stabilized_epoch / value_epoch / stale flag
- **Named values with formula support** — name validation, references between names and cells
- **Externally-driven execution** — no internal threads, caller controls all timing
- **Auto/Manual recalc modes** — correct state machine with epoch tracking
- **Lambda/LET/MAP** — first-class functions, closures, local variable binding
- **External data simulation** — STREAM with periodic counter advancement
- **Cell formatting as metadata** — clean separation: format never affects calculation
- **Serialization round-trip** — bulk enumeration sufficient for save/load
- **Clean API boundary** — C-style spec with typed setters, iterators, per-handle error state

## The 10 Missing Excel Engine Feature Areas

Ranked by importance to a spreadsheet engine pathfinder — not by implementation
difficulty or user demand, but by **how much fundamental engine architecture
they force you to confront**.

### 1. Structural Mutations and Reference Adjustment — DONE

**Status:** Implemented. Insert/delete row/column with full reference rewriting,
`RefFlags` (absolute/relative per axis), `rewrite_expr()` AST walker,
`expr_to_formula()` reconstruction, `#REF!` on invalidation. 32 tests.

**What was missing:** No insert/delete row/column. No absolute references
(`$A$1`). The `CellRef` had no relative/absolute flag. No reference-rewriting
pass over stored formula ASTs.

**Why it matters for the pathfinder:** This is the single hardest engine problem
in spreadsheets. When you insert a row, every formula in the sheet must be
inspected and potentially rewritten. References must be classified as Preserved,
Shifted, Expanded, Contracted, or Invalidated. The address model must
distinguish `A1` (shifts) from `$A$1` (doesn't shift) from `$A1` (column
fixed, row shifts). This forces you to confront stable identity vs. coordinate
addressing — the very problem the Foundation's `RowId`/`ColId` model exists to
solve.

**Foundation says:** §6.1 explicitly requires "one structural rewrite path" and
§3.14 defines the rewrite semantics in detail.

---

### 2. Iterative Calculation for Circular References — DONE

**Status:** Implemented. Tarjan SCC decomposition in `deps.rs`, per-SCC
iteration with configurable `max_iterations` and `convergence_tolerance`,
`IterationConfig` on Engine, `iterating_prev` in EvalContext for cycle-tolerant
evaluation. 11 tests.

**What was missing:** Cycles were hard errors. No iterative convergence mode. No
max_iterations or tolerance parameters. No SCC-based iteration.

**Why it matters:** Excel supports intentional circular references resolved by
bounded iteration (default: 100 iterations, 0.001 convergence). Many financial
models depend on this (interest that depends on balance that depends on
interest). The engine must decompose strongly-connected components and iterate
each SCC independently. This is a fundamentally different recalculation strategy
from a single-pass topological sort.

**Foundation says:** §3.16 specifies SCC-based cycle detection with
profile-selectable CycleError vs Iterative mode. §6.1 requires "SCC iteration"
in the formal-core traces.

---

### 3. Incremental/Dirty-Flag Recalculation — DONE

**Status:** Implemented. `dirty_cells` / `dirty_names` / `full_recalc_needed`
tracking on Engine, reverse dependency map (`reverse_deps`), dirty closure
propagation, `recalculate()` dispatches to incremental or full path,
`last_eval_count()` for observability. Value-only changes use incremental path;
formula structure changes fall back to full recalc. 9 tests.

**What was missing:** Every `recalculate()` rebuilt the full `CalcTree` from
scratch and evaluated every formula cell. No dirty-flagging. No partial recalc.
No incremental dependency graph maintenance.

**Why it matters:** This is the scalability architecture. Excel's smart recalc
engine tracks which cells changed and only walks the dirty subtree of the
dependency graph. The calculation chain is maintained incrementally, not rebuilt
on each pass. While the 63x254 grid is intentionally small, the *architecture*
of dirty tracking — how you propagate invalidation through the dependency graph,
how you handle volatile cells cascading into non-volatile dependents, how you
decide when a subtree can be skipped — is a fundamental engine design problem.

**Foundation says:** §3.4 specifies "Incremental recompute based on dependency
closure" and §3.4.1 describes the incremental graph invariant model with
`necessary`, `stale`, `height`, and `scope` node-level invariants.

---

### 4. The Full Number Format Code System — OPEN

**What's missing:** Format is `Option<u8>` decimals + bold + italic + palette
color. No format code strings. No `positive;negative;zero;text` sections. No
date/time patterns. No percentage, currency, scientific notation display. No
`TEXT()` function.

**Why it matters:** Excel's number format system is a mini-language that the
engine must interpret — it's not a UI concern because the `TEXT()` function uses
it to produce calculated values. Format codes interact with the date serial
number system (gap #5). Any file format compatibility requires parsing and
emitting format code strings. The format code grammar has conditionals, color
codes, thousands separators, locale-dependent symbols, and four-section
conditional logic.

**Foundation says:** Not explicitly scoped in §6 pathfinder, but essential for
any file format compatibility work.

---

### 5. Date/Time Serial Number System — OPEN

**What's missing:** No date value type. `NOW()` returns a Unix-derived number,
not an Excel serial number. No date functions (DATE, YEAR, MONTH, DAY, TODAY,
EDATE, EOMONTH, NETWORKDAYS, etc.). No Lotus 1-2-3 leap year bug
compatibility.

**Why it matters:** Dates are serial numbers displayed through format codes.
Date arithmetic is just number arithmetic — but the conversion functions, the
1900-vs-1904 date system, and the intentional Feb 29, 1900 bug are all
engine-level concerns. Dates are one of the most frequently used spreadsheet
features and a rich source of compatibility edge cases.

**Foundation says:** Not explicitly in §6 pathfinder scope but falls under "core
expressions" and "VisiCalc-sized formula language."

---

### 6. Comprehensive Type Coercion Rules — OPEN

**What's missing:** `"123" + 1` is an error (text cannot be coerced to number).
Excel returns `124`. Comparison between text and numbers follows simplified
rules. Aggregate functions don't distinguish between direct arguments and range
references for coercion purposes. No awareness of Excel's context-dependent
coercion model.

**Why it matters:** Coercion rules are the invisible engine behavior that
determines whether a spreadsheet "works" for users who learned on Excel. The
rules are context-dependent: arithmetic context coerces text to numbers,
aggregation context skips text in ranges but coerces direct text arguments,
comparison context has its own ordering rules. Getting this wrong produces subtle
calculation differences that are extremely hard to debug.

**Foundation says:** Falls under "Excel interop" (§5 REQ) — "must match the
profile definition" for recalc behaviors, which includes coercion semantics.

---

### 7. Implicit Intersection and the @ Operator — OPEN

**What's missing:** No concept of "single-cell context" vs. "array context." No
`@` implicit intersection operator. `=A1:A10` in a single cell doesn't
intersect — the behavior is undefined. No backward compatibility mode for
pre-dynamic-array formulas.

**Why it matters:** This is the bridge between legacy Excel and dynamic-array
Excel. When a formula references a range in a single-cell context, Excel
implicitly intersects (selects the value from the same row or column). The `@`
operator makes this explicit. DNA VisiCalc has dynamic arrays but no implicit
intersection — meaning it can't correctly handle the most common pattern in
pre-2019 Excel formulas. Also essential for `.xlsx` import where `@` operators
appear in formula strings.

**Foundation says:** Falls under the intersection of "core expressions" and
"Excel interop."

---

### 8. Error Model Completeness and Error-Handling Functions — DONE

**Status:** Implemented. `IFERROR`, `IFNA`, `ISERROR`, `ISNA`, `ISBLANK`,
`ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`, `NA()`. All error variants
present (`#DIV/0!`, `#VALUE!`, `#NAME?`, `#REF!`, `#SPILL!`, `#N/A`, `#NULL!`,
`#NUM!`). Excel-style error tags via `excel_tag()`. 30 tests.

**What was missing:** No `IFERROR`/`IFNA`. No `ISERROR`/`ISBLANK`/`ISTEXT`/
`ISNUMBER` type-testing family. Missing error variants (`#NULL!`, `#NUM!`). No
`ERROR.TYPE` function.

**Why it matters:** `IFERROR` is the single most important defensive-programming
primitive in spreadsheets. Without it, users cannot write robust formulas that
handle edge cases. The IS* family enables type-checking patterns.

**Foundation says:** Falls under "core expressions."

---

### 9. Multi-Sheet References — OPEN

**What's missing:** Single sheet only. No `Sheet2!A1` syntax. No sheet
collection in the engine. No 3D references (`Sheet1:Sheet5!A1`). No cross-sheet
dependency tracking.

**Why it matters:** Multi-sheet is the organizational and computational scaling
dimension. Most non-trivial workloads use multiple sheets. Cross-sheet
references participate in the same dependency graph. Sheet insert/delete/rename
triggers reference rewriting (connecting back to gap #1). Even supporting 2
sheets with basic `Sheet!Cell` syntax would exercise the architecture: adding a
sheet identifier to `CellRef`, a sheet collection to `Engine`, cross-sheet
dependency edges.

**Foundation says:** Not in §6 pathfinder scope (VisiCalc was single-sheet). But
the Foundation architecture (§3.11 GreenWorkbook) assumes multiple sheets. This
might be the right thing to defer but should be a conscious decision.

---

### 10. Lambda Helper Functions (REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY) — OPEN

**What's missing:** LAMBDA, LET, and MAP exist. REDUCE, SCAN, BYROW, BYCOL, and
MAKEARRAY do not. The calling convention infrastructure is in place.

**Why it matters:** LAMBDA and MAP cover transformation. REDUCE is the missing
primitive for arbitrary aggregation — it completes the functional programming
model. BYROW/BYCOL bridge row/column-oriented thinking and array computation.
MAKEARRAY is the generative counterpart to SEQUENCE. Since the lambda
infrastructure already exists, these are relatively incremental — but they
represent the difference between "we have lambdas" and "we have a complete
functional computation model."

**Foundation says:** Falls under "VisiCalc-sized formula language."

---

## Additional Feature: External UDF Registration — DONE

**Status:** Implemented. `UdfHandler` trait, `FnUdf` closure wrapper,
`register_udf()` / `unregister_udf()` on Engine, case-insensitive matching
(uppercase storage), UDFs checked after built-in functions in eval fallthrough,
passed through `EvalContext`. 11 tests.

Not part of the original 10 gaps but identified in the Foundation §6 pathfinder
scope as a required capability. Forces the callback model, threading contract,
and oracle semantics that any real embedding will need.

---

## C API Extension Points

The C API specification is a good definition layer for the current scope. It
will need extension for remaining gaps:

| Future Feature | API Impact |
|---|---|
| Number format codes | `DvcCellFormat` struct gains format code string field |
| Date functions | Transparent (more functions in eval engine) |
| Type coercion | Transparent (behavioral change in eval engine) |
| Implicit intersection | Transparent / minor (eval context awareness) |
| Multi-sheet | Major: sheet handle type, `dvc_sheet_create/destroy`, cell refs gain sheet parameter |
| Lambda helpers | Transparent (more functions in eval engine) |

The API's use of `struct_size` versioning and the iterator pattern provide
forward-compatible extension points. The biggest structural change would be
multi-sheet (new handle type).

## Foundation Alignment Summary

| Foundation §6 Requirement | Status |
|---|---|
| One structural rewrite path (§6.1) | Done |
| SCC iteration (§6.1) | Done |
| External UDF registration (§6) | Done |
| Incremental recompute (§3.4) | Done |
| Error model completeness | Done |
| Formal-core traces and artifacts (§6.1) | Not started |

## Resolved Design Questions

These questions have been resolved and documented in
[ENGINE_DESIGN_NOTES.md](ENGINE_DESIGN_NOTES.md):

1. **Charts and Controls as engine-level entities** — RESOLVED. Controls are
   names with metadata (`ControlDefinition`), acting as source nodes in the
   dependency graph. Charts are sink nodes (`ChartDefinition` + `ChartOutput`),
   computed during topological evaluation. Both participate in incremental
   recalculation through a generalized `NodeId` dependency graph.
   See [ENGINE_DESIGN_NOTES.md §1–§3](ENGINE_DESIGN_NOTES.md#1-generalized-dependency-graph).

2. **STREAM function classification** — RESOLVED. Three-category classification:
   `Standard`, `Volatile`, `ExternallyInvalidated`. STREAM is reclassified as
   externally-invalidated — it only recalculates when `tick_streams()` fires,
   not on every recalculation. The `UdfHandler` trait gains a `volatility()`
   method. See [ENGINE_DESIGN_NOTES.md §4](ENGINE_DESIGN_NOTES.md#4-function-volatility-classification).

3. **Change tracking for API consumers** — RESOLVED. Change journal
   (CalcDelta pathfinder) with `ChangeEntry` enum, epoch-tagged entries,
   opt-in `enable_change_tracking()` / `drain_changes()` API. Forward-compatible
   with Foundation's OpLog/CalcDelta model.
   See [ENGINE_DESIGN_NOTES.md §5](ENGINE_DESIGN_NOTES.md#5-change-journal-calcdelta-pathfinder).
