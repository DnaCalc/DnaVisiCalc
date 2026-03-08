use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::address::{CellRange, CellRef, SheetBounds};
use crate::ast::Expr;
use crate::eval::{EvalContext, RuntimeValue, Value};
use crate::parser::ParseError;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FecCapabilityTag {
    ReferenceResolution,
    CallerContext,
    TimeProvider,
    RandomProvider,
    ExternalProvider,
    LocaleParseFormat,
    FeatureGate,
    ErrorDetailEnrichment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F3eDependencyProfile {
    None,
    RefOnly,
    CallerContext,
    TimeProvider,
    RandomProvider,
    ExternalProvider,
    LocaleProfile,
    Composite,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FecFormulaId {
    Cell(CellRef),
    Name(String),
}

pub type FormulaToken = u128;
pub type EvalSessionId = u128;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct F3ePrepareContext {
    pub profile_id: &'static str,
    pub compatibility_version: &'static str,
    pub locale_profile: &'static str,
    pub feature_gate_profile: &'static str,
}

impl Default for F3ePrepareContext {
    fn default() -> Self {
        Self {
            profile_id: "FEC-MIN-B",
            compatibility_version: "dvc-v0",
            locale_profile: "en-US-invariant",
            feature_gate_profile: "dvc-v0-default",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormulaPlan {
    pub token: FormulaToken,
    pub expr: Rc<Expr>,
    pub static_dependencies: FxHashSet<CellRef>,
    pub required_capabilities: Vec<FecCapabilityTag>,
    pub dependency_profile: F3eDependencyProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FecCapabilityDecision {
    Allowed,
    Denied(FecCapabilityTag),
}

#[derive(Debug, Clone)]
pub struct FecCapabilityView {
    pub required_capabilities: Vec<FecCapabilityTag>,
    pub decision: FecCapabilityDecision,
}

impl FecCapabilityView {
    pub fn supports_required(&self) -> bool {
        matches!(self.decision, FecCapabilityDecision::Allowed)
    }

    pub fn required_capabilities(&self) -> &[FecCapabilityTag] {
        &self.required_capabilities
    }
}

#[derive(Debug, Clone)]
pub enum F3eEvalTarget<'a> {
    Cell(CellRef),
    Name(&'a str),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F3eResultKind {
    Scalar,
    Array,
    ArraySpill,
    Error,
    Lambda,
}

#[derive(Debug, Clone)]
pub enum EvalObservation {
    ReadCell(CellRef),
    ReadName(String),
    ReadSpillChild(CellRef),
    VolatileRead,
    ExternalRead,
}

#[derive(Debug, Clone)]
pub struct EvalTransaction {
    pub session_id: EvalSessionId,
    pub formula_id: FecFormulaId,
    pub token: FormulaToken,
    pub snapshot_epoch: u64,
    pub capability_decision: FecCapabilityDecision,
    pub result_kind: F3eResultKind,
    pub runtime: RuntimeValue,
    pub observations: Vec<EvalObservation>,
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
    pub added_spill_children: FxHashSet<CellRef>,
    pub removed_spill_children: FxHashSet<CellRef>,
}

impl F3eDependencyDelta {
    pub fn is_empty(&self) -> bool {
        self.added_cells.is_empty()
            && self.removed_cells.is_empty()
            && self.added_names.is_empty()
            && self.removed_names.is_empty()
            && self.added_spill_children.is_empty()
            && self.removed_spill_children.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpillShapeDelta {
    None,
    Created {
        anchor: CellRef,
        range: CellRange,
    },
    Resized {
        anchor: CellRef,
        old: CellRange,
        new: CellRange,
    },
    Cleared {
        anchor: CellRef,
        old: CellRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// SPEC: FEC-F3E-TXN-001
/// Transactional Plan-B seam: prepare/execute over explicit request envelopes.
pub trait F3eKernel {
    fn prepare(
        &self,
        formula_text: &str,
        bounds: SheetBounds,
        ctx: &F3ePrepareContext,
    ) -> Result<FormulaPlan, ParseError>;

    fn execute(&self, evaluator: &mut EvalContext<'_>, req: EvalRequest<'_>) -> EvalTransaction;
}

/// SPEC: FEC-F3E-TXN-002
/// Transaction coordinator responsibilities exposed by FEC.
pub trait FecCoordinator {
    fn install_plan(&mut self, formula_id: FecFormulaId, plan: &FormulaPlan) -> FormulaToken;

    fn unregister_formula(&mut self, formula_id: &FecFormulaId);

    fn clear(&mut self);

    fn required_capabilities_for(&self, formula_id: &FecFormulaId) -> &[FecCapabilityTag];

    fn expected_token_for(&self, formula_id: &FecFormulaId) -> Option<FormulaToken>;

    fn capability_view(
        &self,
        formula_id: &FecFormulaId,
        required: &[FecCapabilityTag],
    ) -> FecCapabilityView;

    fn open_session(
        &mut self,
        formula_id: &FecFormulaId,
        expected_token: Option<FormulaToken>,
        snapshot_epoch: u64,
    ) -> EvalSessionId;

    fn commit(&mut self, tx: EvalTransaction) -> CommitResult;
}
