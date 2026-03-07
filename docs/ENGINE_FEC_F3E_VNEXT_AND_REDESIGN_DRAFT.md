# FEC/F3E vNext and Clean-Slate Redesign Draft

## Purpose
Provide two contract drafts based on seam evidence from:
- `artifacts/fec_f3e/seam_trace.log`
- `artifacts/fec_f3e/exams_20260308/dynamic_retargeting_trace.log`
- `artifacts/fec_f3e/exams_20260308/spill_takeover_clearance_trace.log`

Draft A is incremental (`vNext`) and keeps the current three-call shape.
Draft B is a clean-slate redesign (ignore impact elsewhere) with a transactional seam.

---

## A) Incremental vNext Contract (Compatibility-Preserving)

### A1. Design Goals
1. Keep `compile -> declare_dependencies -> evaluate`.
2. Add runtime-observed dependency reporting.
3. Make token lifecycle explicit and update-safe.
4. Publish spill-shape and dependency-delta metadata with results.
5. Define deterministic capability-denial behavior.

### A2. Proposed Data Types
```rust
pub type DependencyToken = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyUpdateStatus {
    RegisteredNew,
    Updated,
    Unchanged,
    Replaced,
    TokenMismatch,
}

#[derive(Debug, Clone, Default)]
pub struct F3eDependencyDeclContext {
    pub prior_token: Option<DependencyToken>,
}

#[derive(Debug, Clone, Default)]
pub struct F3eObservedDependencies {
    pub cells_read: FxHashSet<CellRef>,
    pub names_read: FxHashSet<String>,
    pub spill_children_read: FxHashSet<CellRef>,
}

#[derive(Debug, Clone, Default)]
pub struct F3eDependencyDelta {
    pub added_cells: FxHashSet<CellRef>,
    pub removed_cells: FxHashSet<CellRef>,
    pub added_names: FxHashSet<String>,
    pub removed_names: FxHashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpillShapeDelta {
    None,
    Created { anchor: CellRef, range: CellRange },
    Resized { anchor: CellRef, old: CellRange, new: CellRange },
    Cleared { anchor: CellRef, old: CellRange },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDecision {
    Allowed,
    Denied(FecCapabilityTag),
}

#[derive(Debug, Clone)]
pub struct FecCapabilityView {
    pub required: Vec<FecCapabilityTag>,
    pub provided: FxHashSet<FecCapabilityTag>,
    pub decision: CapabilityDecision,
}

#[derive(Debug, Clone)]
pub struct F3eEvalResultVNext {
    pub runtime: RuntimeValue,
    pub observed: F3eObservedDependencies,
    pub spill_shape_delta: SpillShapeDelta,
    pub dependency_delta: F3eDependencyDelta,
}

#[derive(Debug, Clone)]
pub struct FecPublishedResultVNext {
    pub value: Value,
    pub result_kind: F3eResultKind,
    pub spill_shape_delta: SpillShapeDelta,
    pub dependency_delta: F3eDependencyDelta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F3eResultKind {
    Scalar,
    Array,
    ArraySpill,
    Error,
    Lambda,
}
```

### A3. Proposed Trait Contracts
```rust
pub trait F3eEngineVNext {
    fn compile(
        &self,
        formula_text: &str,
        bounds: SheetBounds,
        ctx: &F3eCompileContext,
    ) -> Result<F3eCompiledFormula, ParseError>;

    fn declare_dependencies(
        &self,
        compiled: &F3eCompiledFormula,
        ctx: &F3eDependencyDeclContext,
    ) -> F3eDeclaredDependencies;

    fn evaluate(
        &self,
        evaluator: &mut EvalContext<'_>,
        target: F3eEvalTarget<'_>,
        ctx: &F3eEvalContext,
    ) -> F3eEvalResultVNext;
}

pub trait FecHostVNext {
    fn capability_view(&self, required: &[FecCapabilityTag]) -> FecCapabilityView;

    fn register_or_update_dependencies(
        &mut self,
        formula_id: FecFormulaId,
        prior_token: Option<DependencyToken>,
        declared: &F3eDeclaredDependencies,
        observed: &F3eObservedDependencies,
        delta: &F3eDependencyDelta,
    ) -> (DependencyToken, DependencyUpdateStatus);

    fn publish_result(
        &self,
        formula_id: &FecFormulaId,
        result: &F3eEvalResultVNext,
    ) -> FecPublishedResultVNext;
}
```

### A4. Normative Behavior Clauses
1. `prior_token` is mandatory for update intent.
   - `None` means register-new intent only.
2. If `prior_token` mismatches host state:
   - return `DependencyUpdateStatus::TokenMismatch`,
   - publish deterministic diagnostic.
3. Capability denial must produce deterministic value-level error:
   - no panic,
   - no partial host registration updates.
4. `publish_result` must include spill-shape and dependency-delta metadata.
5. Runtime-observed dependency sets must be mergeable into host graph state.

### A5. Why this vNext draft matches observed pressure
1. Dynamic reference retargeting (`INDIRECT` / selector-driven `OFFSET`) requires runtime-observed dependency reporting.
2. Spill takeover/clearance requires explicit shape-delta publication.
3. Existing token plumbing exists but lacks update semantics; this draft closes that gap.

---

## B) Clean-Slate Redesign Contract (Ignore External Impact)

### B1. Redesign Approach
Replace ad-hoc compile/declare/evaluate host callbacks with:
1. `prepare` phase (static analysis + capability plan),
2. `execute` phase (deterministic run producing typed observation events),
3. `commit` phase (atomic host apply of value + dependency + shape graph changes).

The seam becomes transaction-based and event-sourced.

### B2. Core Transaction Model
```rust
pub type FormulaToken = u128;
pub type EvalSessionId = u128;

#[derive(Debug, Clone)]
pub struct FormulaPlan {
    pub token: FormulaToken,
    pub static_deps: FxHashSet<CellRef>,
    pub required_caps: Vec<FecCapabilityTag>,
    pub dependency_profile: F3eDependencyProfile,
}

#[derive(Debug, Clone)]
pub struct EvalRequest<'a> {
    pub session_id: EvalSessionId,
    pub formula_id: FecFormulaId,
    pub target: F3eEvalTarget<'a>,
    pub token: FormulaToken,
    pub snapshot_epoch: u64,
    pub capability_view: FecCapabilityView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalObservation {
    ReadCell(CellRef),
    ReadName(String),
    ReadSpillChild(CellRef),
    SpillCreated { anchor: CellRef, range: CellRange },
    SpillResized { anchor: CellRef, old: CellRange, new: CellRange },
    SpillCleared { anchor: CellRef, old: CellRange },
    VolatileRead,
    ExternalRead,
}

#[derive(Debug, Clone)]
pub struct EvalTransaction {
    pub session_id: EvalSessionId,
    pub formula_id: FecFormulaId,
    pub token: FormulaToken,
    pub result_kind: F3eResultKind,
    pub runtime: RuntimeValue,
    pub observations: Vec<EvalObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitStatus {
    Applied,
    RejectedTokenMismatch,
    RejectedCapabilityDenied,
    RejectedSnapshotConflict,
}

#[derive(Debug, Clone)]
pub struct CommitResult {
    pub status: CommitStatus,
    pub new_token: FormulaToken,
    pub dependency_delta: F3eDependencyDelta,
    pub spill_shape_delta: SpillShapeDelta,
    pub value: Value,
}
```

### B3. Clean-Slate Traits
```rust
pub trait F3eKernel {
    fn prepare(
        &self,
        formula_text: &str,
        bounds: SheetBounds,
        ctx: &F3eCompileContext,
    ) -> Result<FormulaPlan, ParseError>;

    fn execute(
        &self,
        evaluator: &mut EvalContext<'_>,
        req: EvalRequest<'_>,
    ) -> EvalTransaction;
}

pub trait FecCoordinator {
    fn open_session(
        &mut self,
        formula_id: &FecFormulaId,
        expected_token: Option<FormulaToken>,
    ) -> EvalSessionId;

    fn capability_view(
        &self,
        formula_id: &FecFormulaId,
        required: &[FecCapabilityTag],
    ) -> FecCapabilityView;

    fn commit(&mut self, tx: EvalTransaction) -> CommitResult;
}
```

### B4. Redesign Guarantees
1. Atomicity:
   - value publication, dependency updates, and spill-shape updates are a single commit decision.
2. Replayability:
   - typed `EvalObservation` stream is fully inspectable and reproducible.
3. Deterministic failure:
   - token mismatch, snapshot conflict, and capability denial are explicit statuses.
4. Call-graph clarity:
   - call graphs become phase-level (`prepare`, `execute`, `commit`) and event-level (`EvalObservation`).

### B5. Migration Advice (if ever attempted)
1. Introduce `F3eEvalResultVNext` first (incremental path).
2. Add event capture and token-update semantics.
3. Only then consider collapsing to transaction seam.

---

## Recommendation
Use Draft A now (incremental vNext), and keep Draft B as a target architecture for a future major boundary revision.
