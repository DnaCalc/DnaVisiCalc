# DNA VisiCalc Core Engine Spec v0

## 1. Purpose
This document defines the current Round-0 core engine compatibility scope.

It is the top-priority repo-local specification for engine behavior.

Repository integration scope (file adapter/TUI/repo crate boundaries) is maintained in `docs/SPEC_v0_INTEGRATION_APPENDIX.md`.

## 2. Core Engine Scope
The engine contract is implementation-independent and covers:
- deterministic, in-memory spreadsheet behavior,
- host-driven mutation and recalculation APIs,
- formula/value semantics,
- structural rewrite semantics,
- volatility/invalidation behavior,
- UDF integration,
- controls/charts/change tracking entities,
- formatting and deterministic bulk enumeration surfaces.

## 3. Normative v0 Engine Requirements

### 3.1 Engine Boundary
- Pure in-memory API.
- Externally-driven execution only (no internal timers/threads/I/O).
- Deterministic behavior for identical inputs and operation sequences.

### 3.2 Sheet and Address Model
- Single worksheet.
- VisiCalc-size bounds by default: `A1..BK254` (63 columns x 254 rows).
- A1 parsing supports:
  - relative refs (`A1`),
  - mixed absolute refs (`$A1`, `A$1`),
  - fully absolute refs (`$A$1`),
  - ranges (`A1:B7`, `A1...B7`).

### 3.3 Cell and Name Inputs
- Cells and names accept numeric, text, formula, and clear operations.
- Names are case-insensitive, normalized uppercase, and validated against identifier/function/cell-ref conflicts.

### 3.4 Formula Surface
- Arithmetic, comparison, concatenation, and logical evaluation.
- Dynamic arrays and spill semantics including spill references (`A1#`).
- Explicit v0 function surface (aligned with current Rust baseline):
  - aggregates: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
  - conditional/error handling: `IF`, `IFERROR`, `IFNA`, `NA`, `ERROR`
  - logical predicates: `AND`, `OR`, `NOT`, `ISERROR`, `ISNA`, `ISBLANK`, `ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`
  - core math/scientific: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - financial/lookup: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`
  - text: `CONCAT`, `LEN`
  - dynamic arrays and lambda family: `SEQUENCE`, `RANDARRAY`, `LET`, `LAMBDA`, `MAP`
  - reference helpers: `INDIRECT`, `OFFSET`, `ROW`, `COLUMN`
  - volatile/stream: `NOW`, `RAND`, `STREAM`
- Stage priority for non-Rust implementations:
  - critical first: `LET`, `LAMBDA`, `MAP`, `INDIRECT`, `OFFSET`, `SEQUENCE`, `RANDARRAY`, `STREAM`
  - lower priority in staged runs: trig/scientific and financial/lookup categories, but still part of the target v0 surface.
- `STREAM` semantics are explicit (not Excel-derived):
  - call shape: `STREAM(period_secs [, LAMBDA(counter)])`,
  - `period_secs` must be a positive number,
  - stream state is per formula cell; return value is that cell's stream counter (starting at `0`),
  - stream counters advance only through host-driven `tick_streams(elapsed_secs)` invalidation,
  - no autonomous/background ticking inside the engine,
  - if lambda is provided, it is applied to the counter and may return scalar or array.
- `LAMBDA`/`MAP` are fully dynamic-array aware:
  - lambda parameters may receive scalar or array values,
  - `MAP` must support array broadcasting across inputs,
  - lambda return values may be arrays and must participate in deterministic spill tiling/broadcast output.
- `INDIRECT` supports both A1 and R1C1 text references.
- `RAND`/`RANDARRAY` outputs remain within bounds and change via small perturbations on explicit recalculation events.

### 3.5 Recalc and Epoch Model
- `committed_epoch`, `stabilized_epoch`, and per-value `value_epoch`.
- `Automatic` and `Manual` recalc modes.
- Internal recalculation strategy is non-normative (dependency graph/tree shape, scheduling, incremental vs full recompute are implementation choices).
- Normative requirement is externally observable behavior: deterministic stabilized outputs and correct epoch/staleness semantics for identical input + API-call sequences.

### 3.6 Structural Rewrite Path (Required)
- Row/column structural mutations are in-scope:
  - `insert_row`, `delete_row`, `insert_col`, `delete_col`.
- Formula/name references are rewritten deterministically.
- Invalidated references are surfaced explicitly (for example `#REF!` behavior).
- Mixed and absolute references must preserve anchoring flags through rewrites; coordinate rewriting under structural edits applies to all reference classes (`A1`, `$A1`, `A$1`, `$A$1`).
- Structural mutation requests use a tri-state outcome model:
  - `Applied`: mutation accepted and committed.
  - `Rejected`: request is valid but cannot be executed due to structural/policy constraints.
  - `Invalid`: request is malformed or out of contract.
- Rejected structural requests are atomic no-ops:
  - no partial mutation,
  - no `committed_epoch` increment,
  - deterministic, user-visible rejection reason.

### 3.7 Iteration and Cycle Handling
- SCC cycle detection remains deterministic.
- Engine supports iterative cycle mode via iteration configuration (`enabled`, max iterations, convergence tolerance).
- When iteration is disabled, circular references follow Excel-style non-iterative behavior:
  - no hard recalculation failure solely due to circularity,
  - circular paths read prior stabilized values when available, otherwise `0.0`.
- Cycle detection is also an observable signal:
  - each recalculation that detects circularity in non-iterative mode emits at least one non-fatal diagnostic notification (for host/UI feedback parity with Excel warning behavior).

### 3.8 Volatility and Invalidation Classes
- Functions/UDFs are classified as:
  - `Standard`,
  - `Volatile`,
  - `ExternallyInvalidated`.
- Volatile refresh and externally-triggered refresh are separate pathways (`invalidate_volatile`, stream ticks, `invalidate_udf`).
- Volatile formulas do not self-tick in the background; they update when recalculation/invalidation is explicitly triggered.

### 3.9 External UDFs
- External UDF registration/unregistration is in scope.
- UDF volatility class participates in invalidation behavior.

### 3.10 Engine Entities Beyond Cells/Names
- Controls are engine-managed named-value entities with metadata.
- Charts are engine-managed sink entities producing computed chart outputs.
- Change tracking is engine-managed via opt-in journal/drain API.

### 3.11 Formatting
- Per-cell metadata formatting is in scope:
  - decimals,
  - bold/italic,
  - foreground/background palette colors.
- Formatting does not change formula semantics.

### 3.12 API-Visible Invariants and Conformance
- API-visible invariants are part of the normative contract for compatibility claims.
- Initial invariant and case registry is defined in `docs/ENGINE_CONFORMANCE_TESTS.md`.
- Property forms and temporal semantics are defined in `docs/ENGINE_FORMAL_PROPERTIES.md`.
- Implementations claiming compatibility should report conformance outcomes against that registry.

## 4. Non-goals (Round 0)
- Multi-sheet workbook semantics.
- OOXML fidelity and full Excel object model compatibility.
- Collaboration/replication protocols.
- VBA runtime hosting.
- Full XLL/COM parity.

## 5. References
- `docs/ENGINE_REQUIREMENTS.md`
- `docs/ENGINE_API.md`
- `docs/ENGINE_CONFORMANCE_TESTS.md`
- `docs/ENGINE_FORMAL_PROPERTIES.md`
- `docs/SPEC_v0_INTEGRATION_APPENDIX.md`
