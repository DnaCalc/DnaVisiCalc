# Engine Requirements — DNA VisiCalc

Formal requirements for the core engine, aligned with `docs/SPEC_v0.md`.

Repository integration-only requirements that are not part of the core engine contract are moved to `docs/ENGINE_REQUIREMENTS_INTEGRATION_APPENDIX.md`.

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

### REQ-CALC-001: Deterministic Recalc Outcomes
For identical starting state and identical API-call sequence, recalculation results are deterministic (values, errors, epochs, and staleness visibility).

### REQ-CALC-002: Recalc Completeness
After recalculation stabilizes, all affected formulas/names reflect current inputs and invalidation events under the active recalc mode.

### REQ-CALC-003: Internal Strategy Is Non-Normative
Dependency graph/tree construction, scheduling order, and incremental-vs-full recomputation strategy are implementation details and not part of the required external contract.

### REQ-CALC-004: Iterative Cycle Mode
Engine exposes iterative cycle configuration (`enabled`, `max_iterations`, `convergence_tolerance`) and applies SCC-based iterative stabilization when enabled.
When iteration is disabled, circular references use Excel-style non-iterative fallback semantics:
- no dependency-status failure solely because a cycle exists,
- circular reads use prior stabilized values when available, otherwise `0.0`.
- circularity must still be surfaced via a non-fatal diagnostic notification channel.

### REQ-CALC-005: Value Types
Evaluation produces typed values including number/text/bool/blank/error.

### REQ-CALC-006: Cell State Query
Cell state query includes computed value, `value_epoch`, and stale visibility (`value_epoch < committed_epoch`).

### REQ-CALC-007: Random Function Stability Profile
`RAND` and `RANDARRAY` outputs:
- stay within their declared bounds,
- change only on explicit recalculation/invalidation events,
- evolve via small deterministic perturbations between recalculations.

### REQ-CALC-008: Normative v0 Function Set
Required function set for v0 conformance (aligned with current Rust baseline):
- aggregates: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
- conditional/error handling: `IF`, `IFERROR`, `IFNA`, `NA`, `ERROR`
- logical predicates: `AND`, `OR`, `NOT`, `ISERROR`, `ISNA`, `ISBLANK`, `ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`
- core math/scientific: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
- financial/lookup: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`
- text: `CONCAT`, `LEN`
- dynamic arrays and lambda family: `SEQUENCE`, `RANDARRAY`, `LET`, `LAMBDA`, `MAP`
- reference helpers: `INDIRECT`, `OFFSET`, `ROW`, `COLUMN`
- volatile/stream: `NOW`, `RAND`, `STREAM`

### REQ-CALC-009: Staged Implementation Priority
For staged non-Rust implementations, the following are critical-first:
- `LET`, `LAMBDA`, `MAP`
- `INDIRECT`, `OFFSET`
- `SEQUENCE`, `RANDARRAY`
- `STREAM`

Trig/scientific and financial/lookup functions may be sequenced later in staged runs, but they remain part of REQ-CALC-008 target parity.

### REQ-CALC-010: STREAM Function Contract
`STREAM` is a project-defined function (not inferred from Excel semantics):
- signature: `STREAM(period_secs [, LAMBDA(counter)])`,
- `period_secs` must be finite and strictly positive,
- stream state is per cell formula location,
- base stream value is an integer-like counter starting at `0`,
- counters advance only via explicit `tick_streams(elapsed_secs)` invalidation flow,
- engine does not autonomously tick streams in background.

If the optional lambda is provided, it is applied to the counter and participates in normal recalc/spill semantics.

## 5. Dynamic Arrays and Spill (REQ-SPILL)

### REQ-SPILL-001: Spill Semantics
Array-valued formulas spill from an anchor into member cells with deterministic placement.

### REQ-SPILL-002: Spill Blocking
Blocked spill placement surfaces explicit spill failure behavior instead of silently overwriting existing inputs.

### REQ-SPILL-003: Spill Queries
Engine provides spill-anchor/range query surfaces for anchor/member lookup.

### REQ-SPILL-004: Lambda/Map Dynamic Array Semantics
`LAMBDA` and `MAP` are fully dynamic-array aware:
- lambda parameters may bind scalar or array values,
- `MAP` supports broadcast of input array shapes,
- lambda returns may be scalar or array and must compose into deterministic spilled output tiling/broadcast,
- resulting arrays participate in spill-reference semantics (`A1#`) like other array-producing formulas.

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
Coordinate rewriting applies to all reference classes (`A1`, `$A1`, `A$1`, `$A$1`) under structural edits; `$` does not freeze coordinates against row/column insertion/deletion.

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

### REQ-STR-007: Single-Reference Axis Rewrite Rules
For row operations on a reference row coordinate `r`:
- `insert_row(at)`:
  - if `r >= at`, rewritten row is `r + 1`,
  - else unchanged.
- `delete_row(at)`:
  - if `r == at`, reference is invalidated (`#REF!`),
  - if `r > at`, rewritten row is `r - 1`,
  - else unchanged.

For column operations on a reference column coordinate `c`:
- `insert_col(at)`:
  - if `c >= at`, rewritten column is `c + 1`,
  - else unchanged.
- `delete_col(at)`:
  - if `c == at`, reference is invalidated (`#REF!`),
  - if `c > at`, rewritten column is `c - 1`,
  - else unchanged.

### REQ-STR-008: Range and Name-Formula Rewrite Rules
- Range endpoints are rewritten independently using REQ-STR-007.
- If any endpoint is invalidated, the containing formula/name reference path surfaces explicit invalid-reference behavior (`#REF!`), not silent coercion.
- If rewritten endpoints invert order, normalization to canonical range order is allowed.
- Name formulas use the same rewrite rules as cell formulas.

### REQ-STR-009: Spill-Reference Rewrite Rules
- Spill anchors referenced via `A1#` are rewritten with the same single-reference rules (REQ-STR-007).
- If the anchor is invalidated, spill-reference evaluation surfaces invalid-reference behavior (`#REF!`).
- If structural edits intersect active spill boundaries that cannot be deterministically rewritten under current policy, the mutation is rejected (`valid-but-rejected`), not partially applied.

### REQ-STR-010: Reject Kind Boundaries
- `STRUCTURAL_CONSTRAINT`: valid request blocked by structural state constraints (for example non-rewritable active spill-boundary intersections).
- `POLICY`: valid request blocked by host/engine policy gates independent of coordinate validity (for example policy-disabled structural edits).

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

### REQ-DELTA-004: Circular-Reference Diagnostics
When non-iterative recalculation detects circularity, the engine emits at least one diagnostic entry tagged to the producing epoch. This entry is non-fatal (recalc status remains success) and is intended for host/UI feedback.

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

## 12. Enumeration Contract (REQ-BULK)

### REQ-BULK-001: Deterministic Enumerators
Engine exposes deterministic iterators for:
- all non-empty cell inputs,
- all names,
- all non-default cell formats.

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

## 14. API-Visible Invariant Set (REQ-INVSET)

### REQ-INVSET-001: Invariant Registry
The engine contract includes an API-visible invariant registry maintained in `docs/ENGINE_CONFORMANCE_TESTS.md`.

### REQ-INVSET-002: Mandatory Initial Invariants
Implementations claiming v0 compatibility must satisfy at least:
- `INV-EPOCH-001`
- `INV-EPOCH-002`
- `INV-CELL-001`
- `INV-DET-001`
- `INV-STR-001`
- `INV-CYCLE-001`

### REQ-INVSET-003: Conformance Reportability
Conformance outcomes are reportable at invariant/case granularity (`pass`/`fail`/`waived`) and are tied to a concrete engine build/version.

### REQ-INVSET-004: Temporal Property Contract
Temporal properties are part of the API-visible contract and use the forms defined in `docs/ENGINE_FORMAL_PROPERTIES.md`.
Implementations claiming v0 compatibility must satisfy at least:
- `TEMP-RECALC-001`
- `TEMP-STREAM-001`
- `TEMP-STREAM-002`
- `TEMP-REJECT-001`
- `TEMP-VOL-001`

## 15. Non-goals for this Engine Contract
- Multi-sheet workbook semantics.
- OOXML read/write compatibility.
- Collaboration replication protocol.
- VBA runtime and macro execution host.
- Full XLL/COM parity.
