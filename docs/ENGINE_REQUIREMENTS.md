# Engine Requirements — DNA VisiCalc

Formal requirements for `dnavisicalc-core`, aligned with `docs/SPEC_v0.md` and the current expanded pathfinder scope.

## 1. Scope
- Single-sheet deterministic spreadsheet engine.
- Externally-driven execution (no internal I/O, no autonomous timers/threads).
- Includes structural rewrites, iterative cycle mode, volatility split, external UDF registration, controls/charts, and change tracking.

## 2. Execution Model (REQ-EXEC)

### REQ-EXEC-001: Externally Driven
All state transitions occur inside caller-initiated API calls.

### REQ-EXEC-002: Epoch State
Engine tracks:
- `committed_epoch`: increments on accepted mutations/invalidation events.
- `stabilized_epoch`: latest epoch whose derived outputs are stabilized.
- Per-value `value_epoch`.

### REQ-EXEC-003: Recalc Modes
- `Automatic`: mutation/invalidation paths may trigger immediate recalc.
- `Manual`: mutation/invalidation marks dirty state; caller invokes recalc explicitly.
- Explicit recalc API is valid in both modes (`force recalc` behavior in Automatic mode).

### REQ-EXEC-004: Determinism
For identical inputs and operation sequences, observable results are deterministic.

## 3. Data Model and Addressing (REQ-DATA)

### REQ-DATA-001: Bounds
Default bounds are 63 columns x 254 rows (`A1..BK254`) with configurable bounds support.

### REQ-DATA-002: Cell Inputs
Cells support number, text, formula, and clear operations.

### REQ-DATA-003: Name Inputs
Names support number, text, formula, and clear operations with case-insensitive uppercase identity.

### REQ-DATA-004: Reference Forms
Formula parser and AST support:
- relative refs (`A1`),
- mixed refs (`$A1`, `A$1`),
- absolute refs (`$A$1`),
- ranges (`A1:B7`, `A1...B7`).

### REQ-DATA-005: Name Validation
Names must be valid identifiers and must not collide with built-in functions, boolean literals, or cell references.

## 4. Recalculation Model (REQ-CALC)

### REQ-CALC-001: Dependency Evaluation
Recalculation follows deterministic dependency closure semantics.

### REQ-CALC-002: Incremental Recompute
Engine supports dirty-closure incremental recompute for targeted mutations/invalidation.

### REQ-CALC-003: Full-Rebuild Fallback
Formula-structure and graph-shape changes may force full dependency rebuild/recalc.

### REQ-CALC-004: Iterative Cycle Mode
Engine exposes iterative cycle configuration (`enabled`, `max_iterations`, `convergence_tolerance`) and applies SCC-based iterative stabilization when enabled.

### REQ-CALC-005: Value Types
Evaluation produces typed values including number/text/bool/blank/error.

### REQ-CALC-006: Cell State Query
Cell state query includes computed value, `value_epoch`, and stale visibility (`value_epoch < committed_epoch`).

### REQ-CALC-007: Random Function Stability Profile
`RAND` and `RANDARRAY` outputs:
- stay within their declared bounds,
- change only on explicit recalculation/invalidation events,
- evolve via small deterministic perturbations between recalculations.

## 5. Dynamic Arrays and Spill (REQ-SPILL)

### REQ-SPILL-001: Spill Semantics
Array-valued formulas spill from an anchor into member cells with deterministic placement.

### REQ-SPILL-002: Spill Blocking
Blocked spill placement surfaces explicit spill failure behavior instead of silently overwriting existing inputs.

### REQ-SPILL-003: Spill Queries
Engine provides spill-anchor/range query surfaces for anchor/member lookup.

## 6. Invalidation and Volatility (REQ-INV)

### REQ-INV-001: Volatility Classes
Built-ins/UDFs are treated as one of:
- `Standard`,
- `Volatile`,
- `ExternallyInvalidated`.

### REQ-INV-002: Volatile Presence Query
`has_volatile_cells()` reports whether volatile formulas exist.

### REQ-INV-003: External Presence Query
`has_externally_invalidated_cells()` reports whether externally-invalidated formulas exist.

### REQ-INV-004: Volatile Invalidation Path
`invalidate_volatile()` marks volatile formulas dirty and propagates through normal recalc flow.
Volatile formulas do not autonomously re-evaluate without an explicit recalc/invalidation trigger.

### REQ-INV-005: Stream Tick Path
`tick_streams(elapsed_secs)` advances stream counters and marks fired stream formulas dirty.
- In automatic mode, recompute may happen immediately.
- In manual mode, caller-triggered recalc remains required for stabilization.

### REQ-INV-006: UDF Invalidation Path
`invalidate_udf(name)` targets formulas/names that call the specified externally-invalidated UDF.

## 7. Structural Mutation Path (REQ-STR)

### REQ-STR-001: Supported Ops
Engine provides:
- `insert_row(at)`,
- `delete_row(at)`,
- `insert_col(at)`,
- `delete_col(at)`.

### REQ-STR-002: Deterministic Rewrite
Structural ops rewrite affected formula and name references deterministically.

### REQ-STR-003: Absolute/Mixed Ref Preservation
Row/column anchoring flags are preserved through rewrites.

### REQ-STR-004: Invalidated Reference Behavior
References targeting removed coordinates are surfaced explicitly as invalid references.

### REQ-STR-005: Bounds Validation
Out-of-range structural positions are rejected without partial mutation.

### REQ-STR-006: Valid-But-Rejected Structural Requests
Structurally constrained requests may be rejected as valid commands that cannot execute.
Rejected outcomes are deterministic atomic no-ops:
- no partial mutation,
- no `committed_epoch` increment,
- explicit rejection reason via API diagnostics.

## 8. External UDFs (REQ-UDF)

### REQ-UDF-001: Registration
Engine supports register/unregister lookup by case-insensitive name identity.

### REQ-UDF-002: Volatility Declaration
UDF handlers can declare volatility class, defaulting to `Standard`.

### REQ-UDF-003: Engine Integration
UDF calls participate in normal dependency, recalculation, and invalidation flows.

## 9. Engine Entities: Controls and Charts (REQ-ENT)

### REQ-ENT-001: Controls
Controls are engine-managed named-value entities with definition metadata and validated value mutation.

### REQ-ENT-002: Charts
Charts are engine-managed sink entities with deterministic computed chart outputs derived from source ranges.

### REQ-ENT-003: Mutation APIs
Engine supports create/remove/query/iterate surfaces for controls and charts.

## 10. Change Tracking (REQ-DELTA)

### REQ-DELTA-001: Opt-in Journal
Engine supports enabling/disabling change tracking with drain semantics.

### REQ-DELTA-002: Entry Shape
Journal entries include typed entities (cell/name/chart/format/spill where applicable) and epoch tagging.

### REQ-DELTA-003: Disable Semantics
Disabling change tracking discards pending undrained entries.

## 11. Formatting (REQ-FMT)

### REQ-FMT-001: Metadata-only
Formatting does not alter formula semantics.

### REQ-FMT-002: Supported Fields
- `decimals` (`None` or `0..9`)
- `bold`
- `italic`
- `fg` palette color
- `bg` palette color

### REQ-FMT-003: Deterministic Enumeration
Non-default format enumeration is deterministic.

## 12. Enumeration and Serialization Handoff (REQ-BULK)

### REQ-BULK-001: Deterministic Enumerators
Engine exposes deterministic iterators for:
- all non-empty cell inputs,
- all names,
- all non-default cell formats.

### REQ-BULK-002: Adapter Sufficiency (Current v2 Scope)
These enumerators plus typed setters are sufficient for current file adapter persistence scope:
- `MODE`,
- `ITER`,
- `DYNARR`,
- `CELL`,
- `NAME`,
- `CONTROL`,
- `CHART`,
- `FMT`.

## 13. Error Model (REQ-ERR)

### REQ-ERR-001: Structured Engine Errors
Mutating/querying APIs return structured errors for invalid address/name/parse/dependency/bounds and related contract violations.

### REQ-ERR-002: Explicit Eval Errors
Evaluation failures surface as explicit value-level errors and remain deterministic.

### REQ-ERR-003: Outcome Classification
Mutation APIs distinguish:
- applied success,
- valid-but-rejected outcomes (constraint/policy),
- invalid/error outcomes.

## 14. Non-goals for this Engine Contract
- Multi-sheet workbook semantics.
- OOXML read/write compatibility.
- Collaboration replication protocol.
- VBA runtime and macro execution host.
- Full XLL/COM parity.
