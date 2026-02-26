# Foundation Document Proposals — DNA VisiCalc Pathfinder Feedback

Structured proposals for Foundation document refinements, derived from the DNA VisiCalc pathfinder implementation experience. Each proposal targets a specific Foundation section with verbatim quotes, specific changes, and pathfinder evidence.

This document does NOT edit Foundation files directly. It is a formal upstream proposal for review through the Foundation's synthesis decision process (accept / adapt / defer / reject with rationale per OPERATIONS.md §8.2).

Cross-references: [ENGINE_DESIGN_NOTES.md](ENGINE_DESIGN_NOTES.md) (primary evidence), [ENGINE_API.md](ENGINE_API.md) (C API evidence), [GAP_ANALYSIS.md](GAP_ANALYSIS.md) (completion status evidence).

---

## Part A: ARCHITECTURE_AND_REQUIREMENTS.md (8 proposals)

---

#### Proposal A1: Concrete CalcDelta Entry Types

**Foundation section:** §1.1 item 3 — "CalcDeltas (Derived Outputs)" (line 12)

**Current text:**
> "Engine produces deltas tagged with version info (epoch/value_epoch) and explicit stale/pending status."

**Proposed change:** Add §3.3.2 "CalcDelta Shape" defining concrete delta entry types, epoch tagging, and drain semantics:

```text
§3.3.2 CalcDelta Shape

CalcDelta entries are typed by the entity that changed:

ChangeEntryKind =
  | CellValue      -- a cell's computed value changed
  | NameValue      -- a named value's computed value changed
  | ChartOutput    -- a chart's computed output changed
  | SpillRegion    -- a spill region was created, resized, or removed
  | CellFormat     -- a cell's format metadata changed

Each entry carries:
  - entity identifier (cell address, name string, chart name, or spill anchor)
  - old and new values (or old/new ranges for spill regions)
  - epoch: the committed_epoch at which the change was produced

Emission semantics:
  - Value-change-only: an entry is emitted only when old != new.
    Recalculating a cell to the same value produces no entry.
  - Opt-in: change tracking is enabled/disabled by the consumer.
    When disabled, no entries accumulate and no comparison overhead is incurred.
  - Drain API: entries accumulate in a buffer and are retrieved by
    a drain call that returns all entries and clears the buffer.
    There is no subscription/callback mechanism.

CalcDeltas are outputs only. They record the results of evaluation,
not the mutations that triggered it (that is the OpLog's role).
```

**Rationale:** The one-line Foundation description is too abstract to implement against. The pathfinder needed concrete types to build journal-driven TUI rendering. The `ChangeEntry` enum in ENGINE_DESIGN_NOTES.md §5 (lines 352–390) and the `DvcChangeType` enum in ENGINE_API.md §1.19 (lines 238–248) both emerged from this gap. The value-change-only and opt-in drain semantics were design decisions made during pathfinder implementation that should be captured at the Foundation level.

**Classification:** Addition

**Dependencies:** None

---

#### Proposal A2: Generalize Pipeline to Heterogeneous NodeId Graph

**Foundation section:** §3.4 — "Calculation Engine Pipeline (conceptual)" (line 64); §3.12 — Layer D description (line 215)

**Current text (§3.4, line 65):**
> "Parse → bind/resolve refs → dependency graph → invalidation closure → schedule → evaluate → commit results."

**Current text (§3.12, line 215):**
> "Layer D (Dependencies): derived evaluation graph over evaluable nodes."

**Proposed change:** Add a NodeId concept to §3.4 making the pipeline explicitly heterogeneous:

```text
§3.4 addition — Heterogeneous Node Identity

The dependency graph operates on a unified node identifier:

NodeId =
  | Cell(CellId)
  | Name(NameId)
  | Chart(ChartId)
  // Future: ExternalRef, Sheet, etc.

All entities that participate in calculation are nodes in one
dependency graph, evaluated in one topological pass. Names are
first-class graph nodes (not a separate pre-pass). Charts are
sink nodes (no outgoing dependency edges). Controls map to
Name nodes with additional metadata.

Layer D's "evaluable nodes" (§3.12) are NodeId values.
The evaluation pipeline (parse → bind → graph → invalidate →
schedule → evaluate → commit) operates over NodeId, not
implicitly over cells alone.
```

**Rationale:** The pipeline description at line 65 is implicitly cell-centric. Layer D (line 215) says "evaluable nodes" but doesn't define what kinds of nodes exist. The pathfinder's cell-only graph forced conservative `full_recalc_needed = true` on every name change. Introducing `NodeId` (ENGINE_DESIGN_NOTES.md §1, lines 18–25) fixed this: name changes now propagate incrementally through the same graph as cell changes. Charts as sink nodes eliminate post-eval fixup passes (ENGINE_DESIGN_NOTES.md §3, lines 155, 203–210).

**Classification:** Addition

**Dependencies:** A7 (controls and charts as engine entities)

---

#### Proposal A3: Companion Dirty-Closure Propagation Model

**Foundation section:** §3.4.1 — "Incremental Graph Invariant Model" (lines 71–74)

**Current text:**
> "Green specifies node-level invariants inspired by production incremental systems: `necessary`, `stale`, `height`, and `scope`. Dynamic dependency rewiring (bind-like behavior) must carry explicit scope invalidation rules and deterministic re-stabilization behavior."

**Proposed change:** Add §3.4.2 documenting a simpler baseline propagation model:

```text
§3.4.2 Baseline Propagation Model (pathfinder-validated)

A valid baseline implementation of incremental recalculation uses:

  dirty_nodes: HashSet<NodeId>     -- nodes needing re-evaluation
  reverse_deps: HashMap<NodeId, HashSet<NodeId>>  -- dependents of each node

Dirty-closure propagation:
  1. A mutation marks source nodes dirty.
  2. Transitive closure over reverse_deps marks all reachable
     downstream nodes dirty.
  3. Topological sort of dirty nodes determines evaluation order.
  4. Nodes are evaluated in order; SCC iteration applies to
     cyclic components within the dirty set.

This model produces equivalent results to the §3.4.1 invariant
model for the pathfinder scope. The §3.4.1 model remains the
normative Green specification for formal verification.

Implementation notes:
  - Formula structure changes (new/changed formulas) require
    full dependency graph rebuild.
  - Value-only changes (cell value set, name value set, external
    signal) use incremental dirty-closure propagation.
  - full_recalc_needed is a fallback flag set when the dependency
    graph itself changes, not on every mutation.
```

**Rationale:** The §3.4.1 model (`necessary`, `stale`, `height`, `scope`) is correct but references production incremental systems (Jane Street Incremental) whose complexity exceeds pathfinder scope. The simpler dirty-set model works and is the actual implementation (GAP_ANALYSIS.md lines 85–89: `dirty_cells` / `dirty_names` / `reverse_deps` / dirty closure propagation). Both approaches produce equivalent observable results. Documenting the baseline prevents future pathfinders from over-engineering.

**Classification:** Addition (companion, not replacement)

**Dependencies:** A2 (for NodeId generalization of dirty_cells/dirty_names)

---

#### Proposal A4: Distinguish ExternallyInvalidated from Volatile Recalculation

**Foundation section:** §3.5 — "External Streaming and RTD-like Behavior" (lines 76–82)

**Current text (line 77):**
> "Pathfinder: `STREAM("topic")` is acceptable and deterministic (epoch-scoped external provider)."

**Proposed change:** Add a subsection to §3.5 clarifying the invalidation model:

```text
§3.5 addition — External Invalidation vs Volatile Recalculation

External functions (STREAM, RTD, externally-invalidated UDFs)
recalculate only on explicit external signal, not on every
volatile tick.

Volatile functions (NOW, RAND, RANDARRAY) recalculate on every
invalidation cycle triggered by the host.

The distinction is:
  - invalidate_volatile() marks all volatile cells dirty.
    Called by the host on a timer or user action.
  - tick_streams() / invalidate_udf(name) marks specific
    externally-invalidated cells dirty. Called when the
    external data source signals new data.

Both paths feed into the same incremental dirty-closure
propagation. The difference is scope: volatile invalidation
is global (all volatile cells), external invalidation is
targeted (specific cells by source identity).

STREAM is externally-invalidated, not volatile.
```

**Rationale:** The pathfinder initially classified STREAM as volatile (ENGINE_DESIGN_NOTES.md §4, line 246: "STREAM is not volatile. It should only recalculate when externally invalidated"). This caused unnecessary re-evaluation on every timer tick. The fix was separating `invalidate_volatile()` from `tick_streams()` — each marks different cell sets dirty. The C API codifies this as `dvc_invalidate_volatile` (ENGINE_API.md line 628) vs `dvc_tick_streams` (line 650) vs `dvc_invalidate_udf` (line 668). The Foundation's current text doesn't make this distinction.

**Classification:** Clarification

**Dependencies:** A5 (three-category volatility model)

---

#### Proposal A5: Formalize Three-Category Volatility Model

**Foundation section:** §3.6 — "External UDFs / XLL-like integration" (lines 84–95)

**Current text (line 86):**
> "external UDF registration (name, arity, flags: volatile, thread-safe)"

**Proposed change:** Replace the binary `volatile` flag with a three-category volatility enum:

```text
§3.6 amendment — Function Volatility Classification

Replace "flags: volatile, thread-safe" with:

  Volatility =
    | Standard                -- recalculates when upstream dependencies change
    | Volatile                -- recalculates on every invalidation cycle
    | ExternallyInvalidated   -- recalculates on explicit external signal

  UDF registration: (name, arity, volatility: Volatility, thread_safe: bool)

  Built-in classification:
    Standard:                SUM, IF, VLOOKUP, all math, etc.
    Volatile:                NOW, RAND, RANDARRAY
    ExternallyInvalidated:   STREAM, future async UDFs

  UdfHandler gains a volatility() method (default: Standard).

The host uses has_volatile_cells() and
has_externally_invalidated_cells() to determine which
invalidation mechanisms to set up.
```

**Rationale:** Binary volatile/non-volatile is insufficient. STREAM and future async UDFs need a distinct category that recalculates only on external signal, not every tick. The three-category model is implemented in ENGINE_DESIGN_NOTES.md §4 (lines 250–254), codified in the C API as `DvcVolatility` (ENGINE_API.md §1.18, lines 226–236), and the `UdfHandler` trait gains `fn volatility(&self) -> Volatility` (ENGINE_DESIGN_NOTES.md lines 293–303).

**Classification:** Correction

**Dependencies:** A4 (external invalidation model)

---

#### Proposal A6: Add OpDefineControl and OpDefineChart to OpKind

**Foundation section:** §3.13 — "OpLog Formal Transition Semantics", OpKind enum (lines 242–249)

**Current text:**
```text
OpKind =
  | OpSetFormula
  | OpSetLiteral
  | OpStructural
  | OpDefineName
  | OpExternalUpdate
  | OpMacroMutation
  | OpCalcControl
```

**Proposed change:** Add two variants:

```text
OpKind =
  | OpSetFormula
  | OpSetLiteral
  | OpStructural
  | OpDefineName
  | OpDefineControl      -- NEW: create/remove/update control definition
  | OpDefineChart        -- NEW: create/remove/update chart definition
  | OpExternalUpdate
  | OpMacroMutation
  | OpCalcControl
```

With payload schemas:

```text
OpDefineControl payload:
  name: string,
  action: Define(kind, min, max, step) | Remove

OpDefineChart payload:
  name: string,
  action: Define(source_range) | Remove
```

**Rationale:** Controls and charts are persistent engine state that must survive round-trip serialization and OpLog replay. Without dedicated op kinds, there is no replayable representation of "define slider RATE with range 0–100" or "define chart SALES over B2:D10". The pathfinder's C API includes `dvc_control_define` / `dvc_control_remove` (ENGINE_API.md §13, lines 911–1005) and `dvc_chart_define` / `dvc_chart_remove` (ENGINE_API.md §14, lines 1007–1097), both of which would map to these ops.

**Classification:** Addition

**Dependencies:** A7 (controls and charts as engine entities)

---

#### Proposal A7: Controls and Charts as Engine Entities

**Foundation section:** New section (proposed §3.6.1 or new §3.x between §3.6 and §3.7)

**Current text:** Controls and charts are not mentioned anywhere in the Foundation architecture document.

**Proposed change:** Add a new section:

```text
§3.x Controls and Charts as Engine Entities

Controls and charts are engine-managed entities that participate
in the dependency graph and incremental recalculation.

Control Definition:
  Controls are names with metadata. A control is a named value
  (NodeId::Name) plus a ControlDefinition specifying:
    kind: Slider | Checkbox | Button
    constraints: min, max, step (kind-dependent)

  Controls are source nodes in the dependency graph. They produce
  values through external input (user interaction). The engine
  owns the control state; the UI reads from and writes to the
  engine.

Chart Definition:
  Charts are sink nodes in the dependency graph. A chart is
  identified by name and defined by:
    source_range: CellRange

  Charts produce structured output (ChartOutput) computed during
  the topological evaluation pass. When source cells change,
  dirty propagation marks the chart node dirty and its output
  is recomputed.

Entity Taxonomy:

  Entity    | Graph Role   | NodeId variant     | Produces
  ----------|-------------|-------------------|----------
  Cell      | Internal    | NodeId::Cell       | Value
  Name      | Source/Int  | NodeId::Name       | Value
  Control   | Source      | NodeId::Name       | Value (via metadata)
  Chart     | Sink        | NodeId::Chart      | ChartOutput

Controls and charts are created/removed through dedicated
operations (OpDefineControl, OpDefineChart) and serialized
as part of the document state.
```

**Rationale:** The pathfinder proved these belong in the engine, not the UI. Controls in the TUI started as `Vec<PanelControl>` owned by the UI, synced to engine names via ad-hoc calls (ENGINE_DESIGN_NOTES.md §2, lines 51–63). This violated the Foundation's "no hidden mutation pathways" doctrine (CHARTER.md §2.2.6). Moving them to the engine with `ControlDefinition` (ENGINE_DESIGN_NOTES.md §2, lines 66–84) and `ChartDefinition` / `ChartOutput` (ENGINE_DESIGN_NOTES.md §3, lines 153–172) makes them participate in the dependency graph and incremental recalculation. The C API codifies this with `DvcControlDef` (ENGINE_API.md §1.15, lines 197–208) and `DvcChartDef` / `DvcChartOutput` (ENGINE_API.md §1.16–1.17, lines 210–224).

**Classification:** Addition

**Dependencies:** A2 (NodeId for graph participation), A6 (op kinds for persistence)

---

#### Proposal A8: Pathfinder Scope Completion Status

**Foundation section:** §6 — "Pathfinder Scope Anchor (DnaVisiCalc)" (lines 417–437)

**Current text (§6.1, lines 428–431):**
> "Required semantics: core expressions, references, deterministic dependency closure, manual/auto recalc, STREAM basics, and one structural rewrite path. Required obligations: core semantics packs, epoch/concurrency invariants, oracle alignment, and basic scaling signature. Required artifacts: capability manifest, conformance report, minimized trace corpus, replay bundles for stream cases, and formal-core traces."

**Proposed change:** Add §6.3 with completion status and track decomposition:

```text
§6.3 Round 0 Completion Status

Track A — Engine Implementation: DONE (exceeded minimum scope)
  Completed:
    - Core expressions, references, dependency closure
    - Manual/auto recalc with epoch model
    - STREAM external provider
    - External UDF registration and invocation
    - One structural rewrite path (insert/delete row/col)
    - Incremental/dirty-flag recalculation
    - Iterative calculation for circular references (SCC-based)
    - Error model completeness (IFERROR, IFNA, IS* family)
    - Dynamic arrays with spill semantics
    - Lambda/LET/MAP first-class functions
  Beyond minimum scope:
    - Controls and charts as engine entities
    - Three-category volatility classification
    - Change journal (CalcDelta pathfinder)
    - C API specification

Track B — Green Formal Artifacts: NOT STARTED
  Outstanding:
    - Lean proofs for core semantics
    - TLA+ verification of epoch/scheduling invariants
    - OCaml CLI oracle
    - Conformance packs
    - Minimized trace corpus
    - Stream replay bundles

Track C — Beyond-Scope Initiatives: PARTIAL
  - C API spec (complete, not required for Round 0)
  - Engine design notes (complete, not required for Round 0)

Round 0 exit blocker: Track B.
Track A has exceeded its minimum scope; Track B has not started.
```

**Rationale:** The asymmetric completion profile needs visibility. The pathfinder engine implementation exceeds the §6.1 minimum scope (GAP_ANALYSIS.md "Foundation Alignment Summary" table, lines 276–283: structural rewrite DONE, SCC iteration DONE, UDF DONE, incremental recalc DONE), but the formal artifacts (Lean, TLA+, OCaml oracle, conformance packs) have not started. This asymmetry is a planning signal — Track B is the Round 0 exit blocker, not Track A.

**Classification:** Addition

**Dependencies:** None

---

## Part B: CHARTER.md (2 proposals)

---

#### Proposal B1: Acknowledge Round 0 Engine Achievements

**Foundation section:** §3 — "Program Structure and Names" (lines 46–59)

**Current text (§3.1, lines 48–52):**
> "DNA VisiCalc (`DnaVisiCalc`) — Round 0 pathfinder."

**Proposed change:** Add a status annotation to §3.1:

```text
- **DNA VisiCalc** (`DnaVisiCalc`) — Round 0 pathfinder.
  Status: Engine implementation exceeds minimum scope (structural
  rewrites, incremental recalc, SCC iteration, UDF registration,
  C API spec). Green formal artifacts (Lean, TLA+, OCaml oracle,
  conformance packs) not yet started. See ARCHITECTURE_AND_REQUIREMENTS.md
  §6.3 for detailed completion status.
```

**Rationale:** The CHARTER lists program names but provides no status visibility. A brief annotation makes the asymmetric completion profile visible at the program-structure level, directing readers to the detailed §6.3 status (proposed in A8).

**Classification:** Addition

**Dependencies:** A8 (§6.3 completion status)

---

#### Proposal B2: Glossary Additions

**Foundation section:** §5 — "Glossary (short)" (lines 70–85)

**Current text:** 15 glossary entries covering Profile, OpLog, Epoch, CalcDeltas, etc.

**Proposed change:** Add the following terms, contingent on acceptance of the Part A proposals they depend on:

```text
- **NodeId**: A typed identifier for entities participating in the
  dependency graph. Variants: Cell, Name, Chart. (See
  ARCHITECTURE_AND_REQUIREMENTS.md §3.4.)
- **ExternallyInvalidated**: A volatility category for functions
  that recalculate only on explicit external signal, not on
  every volatile tick. Distinct from Standard (upstream change)
  and Volatile (every cycle). (See ARCHITECTURE_AND_REQUIREMENTS.md §3.6.)
- **ControlDefinition**: Engine-managed metadata for a named value
  acting as a UI control (slider, checkbox, button). Controls are
  source nodes in the dependency graph. (See
  ARCHITECTURE_AND_REQUIREMENTS.md §3.x.)
- **ChartDefinition**: Engine-managed definition of a chart as a
  sink node in the dependency graph, consuming cell values and
  producing structured ChartOutput. (See
  ARCHITECTURE_AND_REQUIREMENTS.md §3.x.)
- **ChangeEntry**: A typed CalcDelta entry recording what changed
  as a result of evaluation. Variants: CellValue, NameValue,
  ChartOutput, SpillRegion, CellFormat. (See
  ARCHITECTURE_AND_REQUIREMENTS.md §3.3.2.)
- **Dirty closure**: The transitive closure of nodes requiring
  re-evaluation after a mutation, computed over reverse dependency
  edges. (See ARCHITECTURE_AND_REQUIREMENTS.md §3.4.2.)
```

**Rationale:** These terms are used throughout the pathfinder design documents and C API specification. Adding them to the glossary ensures consistent usage across Foundation and pathfinder artifacts.

**Classification:** Addition

**Dependencies:** A1 (ChangeEntry), A2 (NodeId), A4/A5 (ExternallyInvalidated), A7 (ControlDefinition, ChartDefinition), A3 (dirty closure)

---

## Part C: OPERATIONS.md (3 proposals)

---

#### Proposal C1: Update Obligation Pack Status

**Foundation section:** §4.1 — "Packs" (lines 64–77)

**Current text (lines 66–77):**
> Packs listed as examples: `PACK.visicalc.core`, `PACK.concurrent.epochs`, `PACK.udf.basic`, `PACK.lean.ocaml.alignment.core`, `PACK.stream.basic`, `PACK.stream.oracle.diff`, `PACK.structural.insert`, etc.

**Proposed change:** Add status annotations for packs exercised by the pathfinder:

```text
Packs exercised by the DnaVisiCalc pathfinder (Track A only):

  PACK.visicalc.core           Exercised (engine tests, not Green-validated)
  PACK.udf.basic               Exercised (11 tests, UdfHandler trait)
  PACK.stream.basic            Exercised (STREAM with tick_streams)
  PACK.structural.insert       Exercised (32 tests, insert/delete row/col)

"Exercised" means the engine implementation covers the pack's
semantic scope and has Rust-level tests. It does NOT mean the
pack itself exists as a Green artifact — Green validation
(conformance packs, oracle alignment, formal traces) remains
outstanding.
```

**Rationale:** The Foundation lists packs as abstract examples. The pathfinder has exercised several of these semantically (GAP_ANALYSIS.md lines 276–283), but the distinction between "engine tests exist" and "Green pack is passing" is critical. Making this explicit prevents premature readiness claims.

**Classification:** Clarification

**Dependencies:** None

---

#### Proposal C2: New Obligation Packs for Pathfinder-Derived Entities

**Foundation section:** §4.1 — "Packs" (lines 64–77)

**Current text:** No packs for controls, charts, change tracking, or volatility classification.

**Proposed change:** Define new packs:

```text
Additional packs (derived from pathfinder, defined for Round 1):

  PACK.control.basic           Control definition, value validation,
                               dependency graph participation
  PACK.chart.basic             Chart definition, output computation,
                               sink node in dependency graph
  PACK.calcdelta.basic         Change journal entry types, epoch tagging,
                               value-change-only emission, drain API
  PACK.volatility.three_cat    Three-category classification (Standard,
                               Volatile, ExternallyInvalidated),
                               invalidation scope behavior

These packs are not required for Round 0 exit but are defined
here so that Round 1 can reference them. Their semantic scope
is documented in ENGINE_DESIGN_NOTES.md and ENGINE_API.md.
```

**Rationale:** The pathfinder produced entities and classifications (controls, charts, CalcDelta, volatility) that need pack definitions for future Green validation. Defining the packs now, even if Round 0 doesn't require them, ensures Round 1 has clear targets.

**Classification:** Addition

**Dependencies:** A1 (CalcDelta), A5 (volatility), A7 (controls and charts)

---

#### Proposal C3: Note Pathfinder-Derived Artifacts

**Foundation section:** §10 — "Round Progression and Exit Coupling" (lines 194–206)

**Current text (lines 196–200):**
> "Minimum exit artifacts per round include: capability manifest, conformance report, updated minimized regression corpus, pack result index for required profiles."

**Proposed change:** Add a note listing artifacts produced beyond the minimum set:

```text
§10 addition — Pathfinder-Produced Artifacts (beyond minimum)

The DnaVisiCalc pathfinder has produced artifacts beyond the
minimum exit set. These are not required for Round 0 exit but
represent reusable design knowledge:

  ENGINE_API.md            C API specification (143 functions)
  ENGINE_DESIGN_NOTES.md   Design specs for controls, charts,
                           volatility classification, change journal
  GAP_ANALYSIS.md          10-gap analysis with Foundation alignment
  FOUNDATION_PROPOSALS.md  This document (upstream proposals)

These artifacts are inputs to Round 1 design and may inform
Green pack development.
```

**Rationale:** §10 specifies minimum exit artifacts but has no mechanism for recording artifacts that exceed the minimum. The pathfinder produced substantial design documentation that would otherwise be invisible to the Foundation-level planning process.

**Classification:** Addition

**Dependencies:** None

---

## Part D: Scope and Process (2 proposals)

---

#### Proposal D1: Handle Asymmetric Scope Completion

**Foundation section:** ARCHITECTURE_AND_REQUIREMENTS.md §6.1 (lines 428–431) and OPERATIONS.md §10 (lines 194–201)

**Current text (OPERATIONS.md line 201):**
> "Round transitions are blocked when required artifacts or pack obligations are missing."

**Proposed change:** Add explicit track decomposition to the round exit criteria:

```text
Round 0 exit criteria are decomposed into parallel tracks:

  Track A — Engine Implementation
    Minimum: core expressions, references, dependency closure,
    manual/auto recalc, STREAM basics, one structural rewrite path.
    Status: DONE (exceeded scope).

  Track B — Green Formal Artifacts
    Minimum: Lean proofs (core semantics), TLA+ (epoch/scheduling),
    OCaml oracle, conformance packs, minimized trace corpus,
    stream replay bundles, formal-core traces.
    Status: NOT STARTED.

  Track C — Beyond-Scope Initiatives
    Not required for exit. Captured for Round 1 planning.

Round 0 exit requires Track A DONE + Track B DONE.
Track C is informational.

The current state is: Track A complete, Track B blocking.
This asymmetry is a planning signal, not a failure —
the pathfinder's purpose was to exercise Track A first.
```

**Rationale:** The Foundation's exit criteria (§10, line 201) are binary — all or nothing. The pathfinder revealed that completion is asymmetric: engine implementation exceeds scope while formal artifacts haven't started. Decomposing into tracks makes the planning signal visible and prevents either premature exit claims or false urgency about Track A.

**Classification:** Addition

**Dependencies:** A8 (§6.3 completion status)

---

#### Proposal D2: Pathfinder Feedback Loop as Process Pattern

**Foundation section:** OPERATIONS.md — new subsection under §8 or §10

**Current text:** §8 covers prompt, research, and synthesis runs. No section describes the pathfinder feedback pattern.

**Proposed change:** Document the pattern:

```text
§8.x (or §10.x) Pathfinder Feedback Pattern

Pathfinder implementations follow a feedback loop:

  1. Implement against Foundation spec
  2. Discover gaps, ambiguities, and missing concepts
  3. Resolve locally (design notes, API specs)
  4. Propose spec refinements upstream (proposals document)

The upstream proposal document:
  - Does NOT edit Foundation files directly
  - Uses per-suggestion structure suitable for accept/adapt/defer/reject
  - Cites specific Foundation sections with line numbers
  - Provides pathfinder evidence (implementation files, test counts)
  - Classifies each proposal (Clarification / Addition / Correction)
  - Notes inter-proposal dependencies

This is distinct from synthesis runs (§8.2) in that the input
is implementation experience, not prompt/research outputs. The
output format is the same: structured proposals for synthesis
decision processing.

FOUNDATION_PROPOSALS.md is an instance of this pattern.
```

**Rationale:** This is what FOUNDATION_PROPOSALS.md itself demonstrates. The pattern of implement → discover gaps → resolve locally → propose upstream is a repeatable process that future pathfinders (DnaPreCalc, DnaSuperCalc) will also follow. Documenting it as a process pattern ensures consistency and sets expectations for what pathfinder feedback looks like.

**Classification:** Addition

**Dependencies:** None

---

## Cross-Reference Matrix

| Proposal | Depends On | Depended On By |
|----------|-----------|---------------|
| A1 | — | B2, C2 |
| A2 | A7 | A3, B2 |
| A3 | A2 | B2 |
| A4 | A5 | — |
| A5 | — | A4, B2, C2 |
| A6 | A7 | — |
| A7 | A2 | A6, B2, C2 |
| A8 | — | B1, D1 |
| B1 | A8 | — |
| B2 | A1, A2, A3, A4/A5, A7 | — |
| C1 | — | — |
| C2 | A1, A5, A7 | — |
| C3 | — | — |
| D1 | A8 | — |
| D2 | — | — |

## Evidence Source Index

| Evidence File | Proposals Citing It |
|--------------|-------------------|
| ENGINE_DESIGN_NOTES.md §1 (NodeId) | A2, A3 |
| ENGINE_DESIGN_NOTES.md §2 (Controls) | A7 |
| ENGINE_DESIGN_NOTES.md §3 (Charts) | A2, A7 |
| ENGINE_DESIGN_NOTES.md §4 (Volatility) | A4, A5 |
| ENGINE_DESIGN_NOTES.md §5 (Change Journal) | A1 |
| ENGINE_API.md §1.15–1.17 (Control/Chart types) | A6, A7 |
| ENGINE_API.md §1.18 (DvcVolatility) | A5 |
| ENGINE_API.md §1.19 (DvcChangeType) | A1 |
| ENGINE_API.md §7 (Invalidation functions) | A4 |
| ENGINE_API.md §13–14 (Control/Chart functions) | A6 |
| ENGINE_API.md §16 (Change tracking) | A1 |
| GAP_ANALYSIS.md lines 83–106 (Incremental recalc) | A3 |
| GAP_ANALYSIS.md lines 276–283 (Alignment summary) | A8, C1 |
| GAP_ANALYSIS.md lines 285–307 (Resolved questions) | A7, A4, A1 |
