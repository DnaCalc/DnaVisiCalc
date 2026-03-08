use std::fmt;
use std::hash::Hasher;
use std::rc::Rc;

use rustc_hash::{FxHashSet, FxHasher};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FecNameId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FecRangeId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FecFormulaStableId(pub u128);

impl fmt::Display for FecNameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:032x}", self.0)
    }
}

impl fmt::Display for FecRangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:032x}", self.0)
    }
}

impl fmt::Display for FecFormulaStableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:032x}", self.0)
    }
}

impl FecNameId {
    pub fn from_canonical_name(name: &str) -> Self {
        let upper = name.trim().to_ascii_uppercase();
        let lo = hash64(&[upper.as_bytes(), b"|name-id|"]);
        let hi = hash64(&[b"|name-id-hi|", upper.as_bytes()]);
        Self(((hi as u128) << 64) | lo as u128)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FecFormulaId {
    Cell(CellRef),
    Name(FecNameId),
}

impl FecFormulaId {
    pub fn from_name(name: &str) -> Self {
        Self::Name(FecNameId::from_canonical_name(name))
    }

    pub fn stable_id(self) -> FecFormulaStableId {
        match self {
            Self::Cell(cell) => {
                let lo = hash64(&[
                    &cell.col.to_le_bytes(),
                    &cell.row.to_le_bytes(),
                    b"|formula-cell|",
                ]);
                let hi = hash64(&[
                    b"|formula-cell-hi|",
                    &cell.col.to_le_bytes(),
                    &cell.row.to_le_bytes(),
                ]);
                FecFormulaStableId(((hi as u128) << 64) | lo as u128)
            }
            Self::Name(id) => {
                let lo = hash64(&[&id.0.to_le_bytes(), b"|formula-name|"]);
                let hi = hash64(&[b"|formula-name-hi|", &id.0.to_le_bytes()]);
                FecFormulaStableId(((hi as u128) << 64) | lo as u128)
            }
        }
    }
}

pub fn spill_range_id(anchor: CellRef, range: CellRange) -> FecRangeId {
    let lo = hash64(&[
        &anchor.col.to_le_bytes(),
        &anchor.row.to_le_bytes(),
        &range.start.col.to_le_bytes(),
        &range.start.row.to_le_bytes(),
        &range.end.col.to_le_bytes(),
        &range.end.row.to_le_bytes(),
        b"|spill-range|",
    ]);
    let hi = hash64(&[
        b"|spill-range-hi|",
        &range.start.col.to_le_bytes(),
        &range.start.row.to_le_bytes(),
        &range.end.col.to_le_bytes(),
        &range.end.row.to_le_bytes(),
    ]);
    FecRangeId(((hi as u128) << 64) | lo as u128)
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
    Name { id: FecNameId, label: &'a str },
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
    ReadName(FecNameId),
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
    pub names_read: FxHashSet<FecNameId>,
    pub spill_children_read: FxHashSet<CellRef>,
}

#[derive(Debug, Clone, Default)]
pub struct F3eDependencyDelta {
    pub added_cells: FxHashSet<CellRef>,
    pub removed_cells: FxHashSet<CellRef>,
    pub added_names: FxHashSet<FecNameId>,
    pub removed_names: FxHashSet<FecNameId>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpillInvalidationScope {
    None,
    EnteredExitedCells,
    PreviousAndCurrentRanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpillBlockCause {
    RuntimeError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpillDeltaEvent {
    None,
    SpillTakeover {
        anchor: CellRef,
        previous_range_id: Option<FecRangeId>,
        new_range_id: FecRangeId,
        old_range: Option<CellRange>,
        new_range: CellRange,
        entered_cells: Vec<CellRef>,
        exited_cells: Vec<CellRef>,
        invalidation_scope: SpillInvalidationScope,
    },
    SpillClearance {
        anchor: CellRef,
        cleared_range_id: FecRangeId,
        old_range: CellRange,
        exited_cells: Vec<CellRef>,
        invalidation_scope: SpillInvalidationScope,
    },
    SpillBlocked {
        anchor: CellRef,
        attempted_range_id: Option<FecRangeId>,
        attempted_range: Option<CellRange>,
        block_cause: SpillBlockCause,
        invalidation_scope: SpillInvalidationScope,
    },
}

impl SpillDeltaEvent {
    pub fn entered_cells(&self) -> &[CellRef] {
        match self {
            Self::SpillTakeover { entered_cells, .. } => entered_cells,
            Self::SpillClearance { .. } | Self::SpillBlocked { .. } | Self::None => &[],
        }
    }

    pub fn exited_cells(&self) -> &[CellRef] {
        match self {
            Self::SpillTakeover { exited_cells, .. } => exited_cells,
            Self::SpillClearance { exited_cells, .. } => exited_cells,
            Self::SpillBlocked { .. } | Self::None => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FecShapeDelta {
    pub spill_event: SpillDeltaEvent,
}

impl Default for FecShapeDelta {
    fn default() -> Self {
        Self {
            spill_event: SpillDeltaEvent::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyImpact {
    None,
    DependencySetChanged,
    SpillRangeChanged,
    SpillBlocked,
}

impl Default for TopologyImpact {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Default)]
pub struct FecTopologyDelta {
    pub dependency_delta: F3eDependencyDelta,
    pub impacted_cells: FxHashSet<CellRef>,
    pub impacted_names: FxHashSet<FecNameId>,
    pub impact: TopologyImpact,
}

impl FecTopologyDelta {
    pub fn is_empty(&self) -> bool {
        self.dependency_delta.is_empty() && matches!(self.impact, TopologyImpact::None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FecValueDelta {
    pub changed: bool,
    pub previous_result_kind: Option<F3eResultKind>,
    pub next_result_kind: F3eResultKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitStatus {
    Applied,
    RejectedSessionNotFound,
    RejectedFormulaNotRegistered,
    RejectedFormulaMismatch,
    RejectedExpectedTokenMismatch,
    RejectedTransactionTokenMismatch,
    RejectedCapabilityNotBound,
    RejectedCapabilityDecisionMismatch,
    RejectedCapabilityDenied,
    RejectedSnapshotConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitRejectCode {
    SessionNotFound,
    FormulaNotRegistered,
    SessionFormulaMismatch,
    ExpectedTokenMismatch,
    TransactionTokenMismatch,
    CapabilityNotBound,
    CapabilityDecisionMismatch,
    SnapshotMismatch,
    CoordinatorSnapshotMismatch,
    CapabilityDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRejectDetail {
    pub code: CommitRejectCode,
    pub expected_token: Option<FormulaToken>,
    pub actual_token: Option<FormulaToken>,
    pub expected_snapshot_epoch: Option<u64>,
    pub actual_snapshot_epoch: Option<u64>,
    pub coordinator_snapshot_epoch: Option<u64>,
    pub denied_capability: Option<FecCapabilityTag>,
}

#[derive(Debug, Clone)]
pub struct CommitResult {
    pub status: CommitStatus,
    pub reject_detail: Option<CommitRejectDetail>,
    pub new_token: FormulaToken,
    pub value: Value,
    pub value_delta: FecValueDelta,
    pub shape_delta: FecShapeDelta,
    pub topology_delta: FecTopologyDelta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FecSeamPerfCounters {
    pub install_plan_count: u64,
    pub open_session_count: u64,
    pub capability_view_count: u64,
    pub commit_count: u64,
    pub commit_applied_count: u64,
    pub commit_rejected_count: u64,
    pub token_rotation_count: u64,
    pub dep_delta_cells_total: u64,
    pub dep_delta_names_total: u64,
    pub dep_delta_spill_children_total: u64,
    pub spill_hint_count: u64,
    pub spill_entered_total: u64,
    pub spill_exited_total: u64,
    pub spill_takeover_count: u64,
    pub spill_clearance_count: u64,
    pub spill_blocked_count: u64,
    pub reject_session_not_found_count: u64,
    pub reject_formula_not_registered_count: u64,
    pub reject_formula_mismatch_count: u64,
    pub reject_expected_token_mismatch_count: u64,
    pub reject_transaction_token_mismatch_count: u64,
    pub reject_capability_not_bound_count: u64,
    pub reject_capability_decision_mismatch_count: u64,
    pub reject_capability_denied_count: u64,
    pub reject_snapshot_mismatch_count: u64,
    pub reject_coordinator_snapshot_mismatch_count: u64,
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
    fn set_coordinator_epoch(&mut self, epoch: u64);

    fn install_plan(&mut self, formula_id: FecFormulaId, plan: &FormulaPlan) -> FormulaToken;

    fn unregister_formula(&mut self, formula_id: &FecFormulaId);

    fn clear(&mut self);

    fn required_capabilities_for(&self, formula_id: &FecFormulaId) -> &[FecCapabilityTag];

    fn expected_token_for(&self, formula_id: &FecFormulaId) -> Option<FormulaToken>;

    fn capability_view(
        &mut self,
        session_id: EvalSessionId,
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

    fn perf_counters(&self) -> FecSeamPerfCounters;

    fn reset_perf_counters(&mut self);
}

fn hash64(parts: &[&[u8]]) -> u64 {
    let mut hasher = FxHasher::default();
    for part in parts {
        hasher.write(part);
    }
    hasher.finish()
}
