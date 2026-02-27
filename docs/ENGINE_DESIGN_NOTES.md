# Engine Design Notes — DNA VisiCalc

Design decisions and specifications for engine features that extend beyond the current implementation. This document captures architectural direction for controls, charts, function classification, and change tracking — resolving the open design questions from [GAP_ANALYSIS.md](GAP_ANALYSIS.md).

Cross-references: [ENGINE_API.md](ENGINE_API.md) (C API specification), [ENGINE_REQUIREMENTS.md](ENGINE_REQUIREMENTS.md) (formal requirements), [C_API_GUIDELINES.md](C_API_GUIDELINES.md) (API design patterns).

---

## 1. Generalized Dependency Graph

### Current State

The dependency graph (`CalcTree` in `deps.rs`) operates on `CellRef` keys only. Formula cells participate as nodes with dependency edges to other cells. Named values are evaluated in a separate pass — any name mutation sets `full_recalc_needed = true`, bypassing incremental recalculation entirely.

### Design: NodeId

The dependency graph should support heterogeneous node types through a unified identifier:

```rust
enum NodeId {
    Cell(CellRef),
    Name(String),
    Chart(String),
    // Future: ExternalRef, Sheet, etc.
}
```

All entities that participate in calculation become nodes in one dependency graph, evaluated in one topological pass. This replaces the current two-pass approach (cells first, then names conservatively).

### Consequences

**Names become first-class graph nodes.** A named formula `TAX_RATE = 0.15` creates a `NodeId::Name("TAX_RATE")` node. Cells referencing `TAX_RATE` have edges to this node. When `TAX_RATE` changes, only its dependents are marked dirty — no `full_recalc_needed` flag.

**Charts are sink nodes** (§3). They depend on their source range cells. When those cells change, the chart node is marked dirty and its output recomputed during the topological eval pass.

**Controls are source nodes** (§2). They produce values through external input. They map to named values — a control IS a name with additional metadata. No new `NodeId` variant is needed for controls; they use `NodeId::Name`.

**Evaluation order:** The topological sort over `NodeId` naturally interleaves cell evaluation, name evaluation, and chart computation. No post-eval fixup pass.

**Incremental propagation:** The existing dirty-closure mechanism (`dirty_cells`, `dirty_names`, `reverse_deps`) generalizes to `dirty_nodes: HashSet<NodeId>` and `reverse_deps: HashMap<NodeId, HashSet<NodeId>>`. A mutation to any node type propagates through the same incremental path.

### Open Questions

- **Migration path:** The `CalcTree` currently stores `BTreeMap<CellRef, CalcNode>`. Migrating to `BTreeMap<NodeId, CalcNode>` is straightforward but touches many call sites. This can be staged: first add `Name` nodes (fixing the conservative full-recalc), then add `Chart` nodes.
- **SCC iteration:** Cyclic SCCs can only contain cells and names (charts are sinks, controls are sources). The SCC iteration logic needs no fundamental change, only generalization of the node type.

---

## 2. Controls as Engine Entities

### Current State

Controls live in the TUI as `Vec<PanelControl>`:

```rust
pub struct PanelControl {
    pub name: String,
    pub kind: ControlKind,   // Slider, Checkbox, Button
    pub value: f64,
}
```

The TUI syncs control values to the engine via `engine.set_name_number(&ctrl.name, ctrl.value)`. The engine has no knowledge that a name is backed by a control — it sees only a named numeric value.

### Design: ControlDefinition

Controls are **names with metadata**. The engine stores a `ControlDefinition` alongside the existing `NameEntry`:

```rust
pub struct ControlDefinition {
    pub kind: ControlKind,
    pub min: f64,          // Slider: minimum value (default 0.0)
    pub max: f64,          // Slider: maximum value (default 100.0)
    pub step: f64,         // Slider: increment step (default 1.0)
}

pub enum ControlKind {
    Slider,
    Checkbox,
    Button,
}
```

The definition is stored in a parallel map: `controls: HashMap<String, ControlDefinition>`, keyed by the uppercase name (same key space as `names`).

### Engine Methods

```rust
/// Define a control. Creates the named value if it doesn't exist,
/// with initial value based on kind (Slider: min, Checkbox: 0.0, Button: 0.0).
/// If the name already exists as a plain name, it is promoted to a control.
pub fn define_control(&mut self, name: &str, def: ControlDefinition) -> Result<(), EngineError>;

/// Remove a control definition. The underlying named value is NOT removed —
/// it reverts to being a plain name. To also remove the value, call clear_name().
pub fn remove_control(&mut self, name: &str) -> bool;

/// Set a control's value. Validates against min/max for sliders, 0.0/1.0 for checkboxes.
/// Equivalent to set_name_number() but with validation.
pub fn set_control_value(&mut self, name: &str, value: f64) -> Result<(), EngineError>;

/// Get a control's current value. Returns None if the name is not a control.
pub fn control_value(&self, name: &str) -> Option<f64>;

/// Get a control's definition. Returns None if the name is not a control.
pub fn control_definition(&self, name: &str) -> Option<&ControlDefinition>;

/// Iterate over all controls in alphabetical order.
pub fn all_controls(&self) -> Vec<(String, ControlDefinition, f64)>;
```

### Validation Rules

| Kind | Constraint |
|------|-----------|
| Slider | `min <= value <= max`, `step > 0` |
| Checkbox | `value` must be `0.0` or `1.0` |
| Button | `value` is always `0.0`; "pressing" is an external action that triggers a name update |

### Dependency Graph Participation

Controls don't need a separate `NodeId` variant. They are names, and names are graph nodes (§1). A slider named `RATE` is `NodeId::Name("RATE")`. Cells containing `=A1 * RATE` depend on this node. When the user moves the slider, `set_control_value("RATE", 0.25)` updates the name, marks `NodeId::Name("RATE")` dirty, and incremental propagation handles the rest.

### TUI Migration

The TUI's `Vec<PanelControl>` becomes a view over the engine's control state. The TUI reads control definitions and values from the engine rather than maintaining its own copy. The `sync_control_to_engine()` call is replaced by direct `engine.set_control_value()`.

```
Before: TUI owns controls → syncs to engine names
After:  Engine owns controls → TUI reads from engine, writes via set_control_value()
```

### Serialization

Controls are serialized as part of the engine state. The file format gains a `[Controls]` section listing control definitions. On load, `define_control()` is called for each entry. The underlying named values are set through the normal name-setting path.

---

## 3. Charts as Engine Entities

### Current State

Charts live in the TUI as `ChartState`:

```rust
pub struct ChartState {
    pub source_range: CellRange,
}
```

The TUI reads cell values from the engine to build `ChartData` for rendering. The engine has no knowledge of charts.

### Design: ChartDefinition and ChartOutput

Charts are **sink nodes** in the dependency graph. They consume cell values and produce structured output.

```rust
pub struct ChartDefinition {
    pub source_range: CellRange,
    // Future: chart type, title, axis labels, etc.
}

pub struct ChartOutput {
    pub labels: Vec<String>,
    pub series: Vec<ChartSeriesOutput>,
}

pub struct ChartSeriesOutput {
    pub name: String,
    pub values: Vec<f64>,
}
```

### Engine Storage

```rust
// In Engine struct:
charts: HashMap<String, ChartDefinition>,
chart_outputs: HashMap<String, ChartOutput>,
```

Charts are identified by name (uppercase, same validation as named values but in a separate namespace — a chart and a name can coexist with the same identifier).

### Engine Methods

```rust
/// Define a chart. Creates a sink node in the dependency graph.
pub fn define_chart(&mut self, name: &str, def: ChartDefinition) -> Result<(), EngineError>;

/// Remove a chart definition and its computed output.
pub fn remove_chart(&mut self, name: &str) -> bool;

/// Get a chart's computed output. Returns None if the chart doesn't exist
/// or hasn't been computed yet.
pub fn chart_output(&self, name: &str) -> Option<&ChartOutput>;

/// Iterate over all chart definitions in alphabetical order.
pub fn all_charts(&self) -> Vec<(String, ChartDefinition)>;
```

### Dependency Graph Participation

A chart `SALES_CHART` with `source_range = B2:D10` creates a `NodeId::Chart("SALES_CHART")` node with dependency edges to every cell in B2:D10. During topological evaluation:

1. Cells B2:D10 are evaluated first (they're upstream)
2. The chart node is reached
3. The engine reads the computed values of B2:D10 and builds `ChartOutput`
4. The output is stored in `chart_outputs`

When cell B3 changes, dirty propagation marks `NodeId::Chart("SALES_CHART")` dirty. On the next recalculation, only the chart and its actual dependents are recomputed.

### Output Computation

The chart output computation follows the same logic currently in the TUI's `build_chart_data()`:

- **Single row:** each column is a data point, column headers (row above) as labels
- **Single column:** each row is a data point, row headers (column to left) as labels
- **Multi-row/col:** first row as category labels, remaining rows as series; first column as series names

This logic moves from the TUI into the engine's evaluation pass.

### TUI Migration

The TUI's `ChartState` becomes a reference to an engine chart. Chart rendering reads `engine.chart_output("SALES_CHART")` instead of scanning cells directly. The TUI no longer needs to understand how to extract series from a cell range — that's the engine's job.

### Serialization

Charts are serialized in a `[Charts]` section. On load, `define_chart()` is called for each entry.

---

## 4. Function Volatility Classification

### Current State

Functions are classified as volatile or non-volatile. The check is in `expr_contains_volatile()`:

```rust
if upper == "NOW" || upper == "RAND" || upper == "STREAM" || upper == "RANDARRAY" {
    return true;
}
```

All four functions are treated identically: `has_volatile_cells()` returns true if any exist, and the caller triggers periodic recalculation.

**Problem:** STREAM is not volatile. It should only recalculate when externally invalidated (its timer fires), not on every recalculation. Lumping it with NOW/RAND causes unnecessary re-evaluation.

### Design: Three Categories

| Category | Name | Recalc Trigger | Examples |
|----------|------|----------------|----------|
| Standard | `Volatility::Standard` | Upstream cells/names change | SUM, IF, VLOOKUP, all math |
| Volatile | `Volatility::Volatile` | Every recalculation | NOW, RAND, RANDARRAY |
| Externally-invalidated | `Volatility::ExternallyInvalidated` | Explicit external signal | STREAM, future async UDFs |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Volatility {
    #[default]
    Standard,
    Volatile,
    ExternallyInvalidated,
}
```

### Behavioral Differences

**Volatile functions** are re-evaluated on every recalculation cycle. `invalidate_volatile()` marks all volatile cells dirty. This is called by the host on a timer (or whenever the host wants a refresh). Since volatile cells produce new values each time, they cascade to all dependents.

**Externally-invalidated functions** are re-evaluated only when explicitly invalidated. `tick_streams()` already does this for STREAM cells — it marks specific cells dirty when their timer fires, not `full_recalc_needed`. The behavioral fix is: remove STREAM from the `expr_contains_volatile()` check and ensure `tick_streams()` marks individual cells dirty (not the global flag).

**Interaction with incremental recalc:**

- Volatile: `invalidate_volatile()` → adds all volatile cells to `dirty_cells` → incremental recalc propagates to dependents
- Externally-invalidated: `tick_streams()` (or `invalidate_udf()`) → adds specific cells to `dirty_cells` → incremental recalc propagates to dependents
- Standard: only dirty when an upstream dependency changes

### External Invalidation with Value

The external invalidation signal can optionally carry a value. For STREAM, the counter value is computed internally by `tick_streams()`. But for future externally-invalidated UDFs (e.g., a function that fetches data from an external service), the invalidation might carry the new result directly:

```rust
/// Invalidate a specific externally-invalidated cell with a new value.
/// The value is stored and returned by the function on the next evaluation.
pub fn invalidate_external(&mut self, cell: CellRef, value: Value);

/// Invalidate all cells using a specific externally-invalidated UDF.
pub fn invalidate_udf(&mut self, name: &str);
```

### UDF Volatility

The `UdfHandler` trait gains a volatility method:

```rust
pub trait UdfHandler: std::fmt::Debug {
    fn call(&self, args: &[Value]) -> Value;

    /// The volatility category of this UDF. Defaults to Standard.
    fn volatility(&self) -> Volatility {
        Volatility::Standard
    }
}
```

A UDF registered with `Volatility::ExternallyInvalidated` is only re-evaluated when the host calls `invalidate_udf("MY_UDF")`, which marks all cells containing calls to that UDF as dirty.

### has_volatile_cells vs has_externally_invalidated_cells

The existing `has_volatile_cells()` should be split:

```rust
/// True if any cell contains NOW, RAND, RANDARRAY, or a Volatile UDF.
/// Host should set up a periodic timer to call invalidate_volatile().
pub fn has_volatile_cells(&self) -> bool;

/// True if any cell contains STREAM or an ExternallyInvalidated UDF.
/// Host should set up appropriate external triggers.
pub fn has_externally_invalidated_cells(&self) -> bool;
```

### Open Questions

- Should externally-invalidated functions also re-evaluate when their non-external arguments change? For example, if `=STREAM(A1)` depends on A1 for its period, should changing A1 re-evaluate STREAM? Current behavior: yes (it re-registers with the new period). This seems correct — the function's parameters participate in normal dependency tracking, but the function's "readiness to produce a new value" is externally triggered.

---

## 5. Change Journal (CalcDelta Pathfinder)

### Foundation Context

The Foundation architecture defines three hard boundaries:

- **OpLog**: All mutations are operations (OpSetFormula, OpSetLiteral, OpStructural, OpDefineName, OpExternalUpdate, OpCalcControl)
- **DocSnapshot**: Immutable versioned state per epoch
- **CalcDeltas**: Engine-produced output deltas tagged with epoch/value_epoch, explicit stale/pending status

The change journal is our pathfinder implementation of **CalcDeltas** — the engine's output channel that tells consumers "what changed as a result of evaluation."

### Separation of Concerns

The Foundation model cleanly separates:

- **OpLog = inputs**: "The user set A1 to 42", "The user inserted row 5"
- **CalcDeltas = outputs**: "B1 changed from 52 to 62", "Chart SALES updated"

The change journal records outputs only. It does not record what mutation caused the change — that's the future OpLog's responsibility. This separation is preserved in the design.

### ChangeEntry Types

```rust
pub enum ChangeEntry {
    /// A cell's computed value changed.
    CellValue {
        cell: CellRef,
        old: Value,
        new: Value,
        epoch: u64,
    },

    /// A named value's computed value changed.
    NameValue {
        name: String,
        old: Value,
        new: Value,
        epoch: u64,
    },

    /// A chart's computed output changed.
    ChartOutput {
        name: String,
        epoch: u64,
    },

    /// A spill region was created, changed, or removed.
    SpillRegion {
        anchor: CellRef,
        old_range: Option<CellRange>,
        new_range: Option<CellRange>,
        epoch: u64,
    },

    /// A cell's format changed (metadata, not a calc result).
    CellFormat {
        cell: CellRef,
        old: CellFormat,
        new: CellFormat,
        epoch: u64,
    },
}
```

### Foundation CalcDelta Mapping

| ChangeEntry variant | Foundation equivalent | What it represents |
|---|---|---|
| `CellValue` | CalcDelta for cell | A cell's computed value changed |
| `NameValue` | CalcDelta for name | A name's computed value changed |
| `ChartOutput` | CalcDelta for chart | A chart's computed output changed |
| `SpillRegion` | Structural CalcDelta | A spill region was created/changed/removed |
| `CellFormat` | Metadata delta | Format changed (not a calc result, but trackable) |

Every entry carries `epoch: u64` — the `committed_epoch` at which the change was produced. This enables the consumer to correlate changes with snapshots, matching the Foundation's epoch-tagged CalcDelta model.

### Engine API

```rust
/// Enable change tracking. Entries accumulate until drained.
pub fn enable_change_tracking(&mut self);

/// Disable change tracking. Pending entries are discarded.
pub fn disable_change_tracking(&mut self);

/// Returns true if change tracking is enabled.
pub fn is_change_tracking_enabled(&self) -> bool;

/// Drain all accumulated change entries. Returns them in the order they were produced.
/// The internal buffer is cleared after draining.
pub fn drain_changes(&mut self) -> Vec<ChangeEntry>;
```

### When Entries Are Produced

| Event | ChangeEntry produced |
|-------|---------------------|
| Recalculation computes a different value for a cell | `CellValue` (if old != new) |
| Recalculation computes a different value for a name | `NameValue` (if old != new) |
| Chart evaluation produces different output | `ChartOutput` |
| Spill region created, changed size, or removed | `SpillRegion` |
| `set_cell_format()` changes a cell's format | `CellFormat` |
| `clear()` called | Entries for every cleared cell/name/chart |

**Important:** Only actual value changes produce entries. If a cell is recalculated and produces the same value, no `CellValue` entry is emitted. This keeps the journal minimal and meaningful.

### Journal Lifecycle

1. Consumer calls `enable_change_tracking()` (opt-in — no overhead when disabled)
2. Engine accumulates `ChangeEntry` items during mutations and recalculations
3. Consumer calls `drain_changes()` to retrieve all entries and clear the buffer
4. Entries are tagged with the epoch that produced them
5. Consumer can call `disable_change_tracking()` to stop accumulating

The journal is a simple append-only buffer. There is no subscription/callback mechanism — the consumer polls via `drain_changes()`. This matches the externally-driven execution model (REQ-EXEC-001).

### Implementation Notes

The change journal is maintained inside the `Engine` struct:

```rust
// In Engine struct:
change_tracking_enabled: bool,
change_journal: Vec<ChangeEntry>,
```

During recalculation, the engine compares old and new values for each evaluated cell/name. If they differ and tracking is enabled, a `ChangeEntry` is appended. The comparison uses `Value::eq` — NaN is not equal to NaN, so a volatile cell producing NaN repeatedly will produce a change entry each time (this is correct: the value is semantically "new" each recalc).

For `CellFormat` entries, the engine captures the old format before applying the new one. For `SpillRegion`, the engine compares the old and new spill ranges after evaluation.

---

## 6. TUI Integration

### Change Journal as Validation

The TUI should be an early consumer of the change journal, validating that the design works for its primary intended use case.

### Current TUI Rendering

The TUI currently:
- Re-renders all visible cells every frame (checking `engine.cell_state()` for each)
- Rebuilds chart data by scanning all cells in the chart's source range every frame
- Checks `engine.committed_epoch()` to detect staleness for the status bar

### Migration: Journal-Driven Updates

With the change journal enabled:

1. **Startup:** `engine.enable_change_tracking()`
2. **After recalculation:** `let changes = engine.drain_changes()`
3. **Cell rendering:** Only cells mentioned in `CellValue` entries need re-rendering. The TUI maintains a `dirty_cells: HashSet<CellRef>` populated from the journal.
4. **Chart rendering:** Only recompute chart data when a `ChartOutput` entry appears for that chart, OR when a `CellValue` entry mentions a cell in the chart's source range.
5. **Format rendering:** Only update cell appearance when a `CellFormat` entry appears.

### Migration: Engine-Backed Controls

```
Before:
  TUI creates PanelControl → calls engine.set_name_number()
  TUI renders from its own PanelControl.value

After:
  TUI calls engine.define_control("RATE", slider_def)
  User adjusts → TUI calls engine.set_control_value("RATE", 0.25)
  TUI renders from engine.control_value("RATE")
```

The TUI's `Vec<PanelControl>` becomes a lightweight view struct that caches the engine's control state for rendering purposes, refreshed from `engine.all_controls()` when control definitions change.

### Migration: Engine-Backed Charts

```
Before:
  TUI stores ChartState with source_range
  TUI scans cells in range to build ChartData
  TUI renders ChartData

After:
  TUI calls engine.define_chart("SALES", chart_def)
  After recalc, TUI reads engine.chart_output("SALES")
  TUI renders from ChartOutput
```

The chart data computation logic currently in `app.rs` moves into the engine's evaluation pass. The TUI becomes a pure renderer of engine-computed chart output.

---

## Summary of Resolved Design Questions

| Question | Resolution | Reference |
|----------|-----------|-----------|
| Controls as engine entities | Controls = names + metadata (ControlDefinition). Source nodes in dependency graph. | §2 |
| Charts as engine entities | Charts = sink nodes in dependency graph. ChartDefinition + ChartOutput. Computed during topological eval. | §3 |
| STREAM is not volatile | Three-category classification: Standard, Volatile, ExternallyInvalidated. STREAM moves to ExternallyInvalidated. | §4 |
| Change tracking for API consumers | Change journal (CalcDelta pathfinder). ChangeEntry enum, epoch-tagged, opt-in drain API. | §5 |
| Dependency graph generalization | NodeId enum (Cell, Name, Chart). One graph, one topological pass, one incremental propagation mechanism. | §1 |
| TUI consuming change journal | Journal-driven rendering: dirty cells from CellValue entries, chart updates from ChartOutput entries. | §6 |
