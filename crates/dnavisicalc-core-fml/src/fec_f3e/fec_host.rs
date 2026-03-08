use rustc_hash::{FxHashMap, FxHashSet};

use crate::address::{CellRange, CellRef};
use crate::eval::{CellError, RuntimeValue, Value};

use super::contracts::{
    CommitRejectCode, CommitRejectDetail, CommitResult, CommitStatus, EvalObservation,
    EvalSessionId, EvalTransaction, F3eDependencyDelta, F3eDependencyProfile,
    F3eObservedDependencies, F3eResultKind, FecCapabilityDecision, FecCapabilityTag,
    FecCapabilityView, FecCoordinator, FecFormulaId, FecSeamPerfCounters, FecShapeDelta,
    FecTopologyDelta, FecValueDelta, FormulaPlan, FormulaToken, SpillBlockCause, SpillDeltaEvent,
    SpillInvalidationScope, TopologyImpact, spill_range_id,
};
use super::spec::FEC_F3E_INTERFACE_VERSION;
use super::trace::{
    boundary_duration_us, boundary_trace_event, boundary_trace_start, format_capabilities,
    format_formula_id, result_kind_name, spill_shape_name,
};

#[derive(Debug, Clone)]
struct FormulaRuntimeState {
    token: FormulaToken,
    required_capabilities: Vec<FecCapabilityTag>,
    dependency_profile: F3eDependencyProfile,
    observed_dependencies: F3eObservedDependencies,
    last_spill_range: Option<CellRange>,
    last_result_kind: Option<F3eResultKind>,
    last_scalar_value: Option<Value>,
}

#[derive(Debug, Clone)]
struct EvalSessionState {
    formula_id: FecFormulaId,
    expected_token: Option<FormulaToken>,
    snapshot_epoch: u64,
    capability_decision: Option<FecCapabilityDecision>,
}

#[derive(Debug)]
pub struct DefaultFecHost {
    formulas: FxHashMap<FecFormulaId, FormulaRuntimeState>,
    sessions: FxHashMap<EvalSessionId, EvalSessionState>,
    next_session_id: EvalSessionId,
    token_nonce: u64,
    provided_capabilities: FxHashSet<FecCapabilityTag>,
    coordinator_epoch: u64,
    perf_counters: FecSeamPerfCounters,
}

impl Default for DefaultFecHost {
    fn default() -> Self {
        Self {
            formulas: FxHashMap::default(),
            sessions: FxHashMap::default(),
            next_session_id: 0,
            token_nonce: 0,
            provided_capabilities: default_provided_capabilities(),
            coordinator_epoch: 0,
            perf_counters: FecSeamPerfCounters::default(),
        }
    }
}

impl DefaultFecHost {
    #[allow(dead_code)]
    pub fn interface_version(&self) -> &'static str {
        FEC_F3E_INTERFACE_VERSION
    }

    #[allow(dead_code)]
    pub fn dependency_profile_for(
        &self,
        formula_id: &FecFormulaId,
    ) -> Option<F3eDependencyProfile> {
        self.formulas
            .get(formula_id)
            .map(|state| state.dependency_profile)
    }

    #[allow(dead_code)]
    pub fn registration_token_for(&self, formula_id: &FecFormulaId) -> Option<FormulaToken> {
        self.formulas.get(formula_id).map(|state| state.token)
    }

    #[allow(dead_code)]
    pub fn observed_dependencies_for(
        &self,
        formula_id: &FecFormulaId,
    ) -> Option<&F3eObservedDependencies> {
        self.formulas
            .get(formula_id)
            .map(|state| &state.observed_dependencies)
    }
}

impl FecCoordinator for DefaultFecHost {
    fn set_coordinator_epoch(&mut self, epoch: u64) {
        self.coordinator_epoch = epoch;
    }

    fn install_plan(&mut self, formula_id: FecFormulaId, plan: &FormulaPlan) -> FormulaToken {
        let trace_start = boundary_trace_start();
        let formula_id_text = format_formula_id(&formula_id);
        let formula_stable_id = formula_id.stable_id();
        let (observed_dependencies, last_spill_range, last_result_kind, last_scalar_value) =
            if let Some(previous) = self.formulas.get(&formula_id) {
                (
                    previous.observed_dependencies.clone(),
                    previous.last_spill_range,
                    previous.last_result_kind,
                    previous.last_scalar_value.clone(),
                )
            } else {
                (F3eObservedDependencies::default(), None, None, None)
            };
        self.formulas.insert(
            formula_id,
            FormulaRuntimeState {
                token: plan.token,
                required_capabilities: plan.required_capabilities.clone(),
                dependency_profile: plan.dependency_profile,
                observed_dependencies,
                last_spill_range,
                last_result_kind,
                last_scalar_value,
            },
        );
        boundary_trace_event(
            "fec.install_plan",
            &[
                ("formula_id", formula_id_text),
                ("formula_stable_id", formula_stable_id.to_string()),
                ("dep_count", plan.static_dependencies.len().to_string()),
                (
                    "required_caps",
                    format_capabilities(&plan.required_capabilities),
                ),
                (
                    "dependency_profile",
                    format!("{:?}", plan.dependency_profile),
                ),
                ("token", plan.token.to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        self.perf_counters.install_plan_count =
            self.perf_counters.install_plan_count.saturating_add(1);
        plan.token
    }

    fn unregister_formula(&mut self, formula_id: &FecFormulaId) {
        self.formulas.remove(formula_id);
        self.sessions
            .retain(|_, session| &session.formula_id != formula_id);
    }

    fn clear(&mut self) {
        self.formulas.clear();
        self.sessions.clear();
        self.next_session_id = 0;
        self.token_nonce = 0;
        self.coordinator_epoch = 0;
    }

    fn required_capabilities_for(&self, formula_id: &FecFormulaId) -> &[FecCapabilityTag] {
        if let Some(state) = self.formulas.get(formula_id) {
            state.required_capabilities.as_slice()
        } else {
            &[]
        }
    }

    fn expected_token_for(&self, formula_id: &FecFormulaId) -> Option<FormulaToken> {
        self.formulas.get(formula_id).map(|state| state.token)
    }

    fn capability_view(
        &mut self,
        session_id: EvalSessionId,
        formula_id: &FecFormulaId,
        required: &[FecCapabilityTag],
    ) -> FecCapabilityView {
        let trace_start = boundary_trace_start();
        let decision = required
            .iter()
            .find(|tag| !self.provided_capabilities.contains(tag))
            .copied()
            .map(FecCapabilityDecision::Denied)
            .unwrap_or(FecCapabilityDecision::Allowed);
        let view = FecCapabilityView {
            required_capabilities: required.to_vec(),
            decision,
        };
        let session_bound = if let Some(session) = self.sessions.get_mut(&session_id) {
            if session.formula_id == *formula_id {
                session.capability_decision = Some(decision);
                true
            } else {
                false
            }
        } else {
            false
        };
        self.perf_counters.capability_view_count =
            self.perf_counters.capability_view_count.saturating_add(1);
        boundary_trace_event(
            "fec.capability_view",
            &[
                ("formula_id", format_formula_id(formula_id)),
                ("formula_stable_id", formula_id.stable_id().to_string()),
                ("session_id", session_id.to_string()),
                ("session_bound", session_bound.to_string()),
                ("required_caps", format_capabilities(required)),
                ("required_caps_count", required.len().to_string()),
                (
                    "provided_caps_count",
                    self.provided_capabilities.len().to_string(),
                ),
                ("coordinator_epoch", self.coordinator_epoch.to_string()),
                (
                    "decision",
                    match decision {
                        FecCapabilityDecision::Allowed => "allowed".to_string(),
                        FecCapabilityDecision::Denied(tag) => format!("denied:{tag:?}"),
                    },
                ),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        view
    }

    fn open_session(
        &mut self,
        formula_id: &FecFormulaId,
        expected_token: Option<FormulaToken>,
        snapshot_epoch: u64,
    ) -> EvalSessionId {
        let trace_start = boundary_trace_start();
        self.next_session_id = self.next_session_id.checked_add(1).unwrap_or(1);
        while self.sessions.contains_key(&self.next_session_id) {
            self.next_session_id = self.next_session_id.checked_add(1).unwrap_or(1);
        }
        let session_id = self.next_session_id;
        self.sessions.insert(
            session_id,
            EvalSessionState {
                formula_id: *formula_id,
                expected_token,
                snapshot_epoch,
                capability_decision: None,
            },
        );
        self.perf_counters.open_session_count =
            self.perf_counters.open_session_count.saturating_add(1);
        boundary_trace_event(
            "fec.open_session",
            &[
                ("formula_id", format_formula_id(formula_id)),
                ("formula_stable_id", formula_id.stable_id().to_string()),
                ("session_id", session_id.to_string()),
                (
                    "expected_token",
                    expected_token
                        .map(|token| token.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                ("snapshot_epoch", snapshot_epoch.to_string()),
                ("coordinator_epoch", self.coordinator_epoch.to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        session_id
    }

    fn commit(&mut self, tx: EvalTransaction) -> CommitResult {
        let trace_start = boundary_trace_start();
        let formula_id_text = format_formula_id(&tx.formula_id);
        self.perf_counters.commit_count = self.perf_counters.commit_count.saturating_add(1);
        let outcome = if let Some(session) = self.sessions.remove(&tx.session_id) {
            if session.formula_id != tx.formula_id {
                reject_commit(
                    CommitStatus::RejectedFormulaMismatch,
                    tx.token,
                    "session formula mismatch",
                    CommitRejectDetail {
                        code: CommitRejectCode::SessionFormulaMismatch,
                        expected_token: session.expected_token,
                        actual_token: Some(tx.token),
                        expected_snapshot_epoch: Some(session.snapshot_epoch),
                        actual_snapshot_epoch: Some(tx.snapshot_epoch),
                        coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                        denied_capability: denied_capability_tag(tx.capability_decision),
                    },
                    tx.result_kind,
                )
            } else if tx.snapshot_epoch != session.snapshot_epoch {
                reject_commit(
                    CommitStatus::RejectedSnapshotConflict,
                    tx.token,
                    "snapshot epoch mismatch",
                    CommitRejectDetail {
                        code: CommitRejectCode::SnapshotMismatch,
                        expected_token: session.expected_token,
                        actual_token: Some(tx.token),
                        expected_snapshot_epoch: Some(session.snapshot_epoch),
                        actual_snapshot_epoch: Some(tx.snapshot_epoch),
                        coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                        denied_capability: denied_capability_tag(tx.capability_decision),
                    },
                    tx.result_kind,
                )
            } else if session.snapshot_epoch != self.coordinator_epoch {
                reject_commit(
                    CommitStatus::RejectedSnapshotConflict,
                    tx.token,
                    "coordinator snapshot epoch mismatch",
                    CommitRejectDetail {
                        code: CommitRejectCode::CoordinatorSnapshotMismatch,
                        expected_token: session.expected_token,
                        actual_token: Some(tx.token),
                        expected_snapshot_epoch: Some(session.snapshot_epoch),
                        actual_snapshot_epoch: Some(tx.snapshot_epoch),
                        coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                        denied_capability: denied_capability_tag(tx.capability_decision),
                    },
                    tx.result_kind,
                )
            } else if let Some(state) = self.formulas.get_mut(&tx.formula_id) {
                if let Some(expected) = session.expected_token {
                    if expected != state.token {
                        reject_commit(
                            CommitStatus::RejectedExpectedTokenMismatch,
                            state.token,
                            "expected token does not match coordinator token",
                            CommitRejectDetail {
                                code: CommitRejectCode::ExpectedTokenMismatch,
                                expected_token: Some(expected),
                                actual_token: Some(state.token),
                                expected_snapshot_epoch: Some(session.snapshot_epoch),
                                actual_snapshot_epoch: Some(tx.snapshot_epoch),
                                coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                                denied_capability: denied_capability_tag(tx.capability_decision),
                            },
                            tx.result_kind,
                        )
                    } else if tx.token != state.token {
                        reject_commit(
                            CommitStatus::RejectedTransactionTokenMismatch,
                            state.token,
                            "transaction token mismatch",
                            CommitRejectDetail {
                                code: CommitRejectCode::TransactionTokenMismatch,
                                expected_token: Some(state.token),
                                actual_token: Some(tx.token),
                                expected_snapshot_epoch: Some(session.snapshot_epoch),
                                actual_snapshot_epoch: Some(tx.snapshot_epoch),
                                coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                                denied_capability: denied_capability_tag(tx.capability_decision),
                            },
                            tx.result_kind,
                        )
                    } else {
                        validate_capability_and_apply(
                            state,
                            &tx,
                            session.capability_decision,
                            self.coordinator_epoch,
                            &mut self.token_nonce,
                        )
                    }
                } else {
                    validate_capability_and_apply(
                        state,
                        &tx,
                        session.capability_decision,
                        self.coordinator_epoch,
                        &mut self.token_nonce,
                    )
                }
            } else {
                reject_commit(
                    CommitStatus::RejectedFormulaNotRegistered,
                    tx.token,
                    "formula plan is not registered",
                    CommitRejectDetail {
                        code: CommitRejectCode::FormulaNotRegistered,
                        expected_token: session.expected_token,
                        actual_token: Some(tx.token),
                        expected_snapshot_epoch: Some(session.snapshot_epoch),
                        actual_snapshot_epoch: Some(tx.snapshot_epoch),
                        coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                        denied_capability: denied_capability_tag(tx.capability_decision),
                    },
                    tx.result_kind,
                )
            }
        } else {
            reject_commit(
                CommitStatus::RejectedSessionNotFound,
                tx.token,
                "session not found for commit",
                CommitRejectDetail {
                    code: CommitRejectCode::SessionNotFound,
                    expected_token: None,
                    actual_token: Some(tx.token),
                    expected_snapshot_epoch: None,
                    actual_snapshot_epoch: Some(tx.snapshot_epoch),
                    coordinator_snapshot_epoch: Some(self.coordinator_epoch),
                    denied_capability: denied_capability_tag(tx.capability_decision),
                },
                tx.result_kind,
            )
        };

        let (spill_entered_count, spill_exited_count) = spill_event_counts(&outcome.shape_delta);
        let reject_code = outcome
            .reject_detail
            .as_ref()
            .map(|detail| format!("{:?}", detail.code))
            .unwrap_or_else(|| "none".to_string());
        let reject_detail_text = outcome
            .reject_detail
            .as_ref()
            .map(format_reject_detail)
            .unwrap_or_else(|| "none".to_string());
        update_perf_counters_from_outcome(&mut self.perf_counters, &outcome);

        boundary_trace_event(
            "fec.commit",
            &[
                ("formula_id", formula_id_text),
                ("formula_stable_id", tx.formula_id.stable_id().to_string()),
                ("session_id", tx.session_id.to_string()),
                ("token", tx.token.to_string()),
                ("result_kind", result_kind_name(tx.result_kind).to_string()),
                ("status", format!("{:?}", outcome.status)),
                ("new_token", outcome.new_token.to_string()),
                ("tx_snapshot_epoch", tx.snapshot_epoch.to_string()),
                ("coordinator_epoch", self.coordinator_epoch.to_string()),
                (
                    "tx_capability_decision",
                    capability_decision_name(tx.capability_decision).to_string(),
                ),
                (
                    "dep_delta_cells",
                    (outcome.topology_delta.dependency_delta.added_cells.len()
                        + outcome.topology_delta.dependency_delta.removed_cells.len())
                    .to_string(),
                ),
                (
                    "dep_delta_names",
                    (outcome.topology_delta.dependency_delta.added_names.len()
                        + outcome.topology_delta.dependency_delta.removed_names.len())
                    .to_string(),
                ),
                (
                    "dep_delta_spill_children",
                    (outcome
                        .topology_delta
                        .dependency_delta
                        .added_spill_children
                        .len()
                        + outcome
                            .topology_delta
                            .dependency_delta
                            .removed_spill_children
                            .len())
                    .to_string(),
                ),
                (
                    "shape_delta",
                    spill_shape_name(&outcome.shape_delta).to_string(),
                ),
                (
                    "topology_impact",
                    topology_impact_name(outcome.topology_delta.impact).to_string(),
                ),
                ("value_changed", outcome.value_delta.changed.to_string()),
                ("spill_entered", spill_entered_count.to_string()),
                ("spill_exited", spill_exited_count.to_string()),
                ("reject_code", reject_code),
                ("reject_detail", reject_detail_text),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        outcome
    }

    fn perf_counters(&self) -> FecSeamPerfCounters {
        self.perf_counters
    }

    fn reset_perf_counters(&mut self) {
        self.perf_counters = FecSeamPerfCounters::default();
    }
}

fn validate_capability_and_apply(
    state: &mut FormulaRuntimeState,
    tx: &EvalTransaction,
    session_decision: Option<FecCapabilityDecision>,
    coordinator_epoch: u64,
    token_nonce: &mut u64,
) -> CommitResult {
    let Some(bound_decision) = session_decision else {
        return reject_commit(
            CommitStatus::RejectedCapabilityNotBound,
            state.token,
            "capability decision not bound to session",
            CommitRejectDetail {
                code: CommitRejectCode::CapabilityNotBound,
                expected_token: Some(state.token),
                actual_token: Some(tx.token),
                expected_snapshot_epoch: Some(tx.snapshot_epoch),
                actual_snapshot_epoch: Some(tx.snapshot_epoch),
                coordinator_snapshot_epoch: Some(coordinator_epoch),
                denied_capability: None,
            },
            tx.result_kind,
        );
    };
    if tx.capability_decision != bound_decision {
        return reject_commit(
            CommitStatus::RejectedCapabilityDecisionMismatch,
            state.token,
            "transaction capability decision mismatches session decision",
            CommitRejectDetail {
                code: CommitRejectCode::CapabilityDecisionMismatch,
                expected_token: Some(state.token),
                actual_token: Some(tx.token),
                expected_snapshot_epoch: Some(tx.snapshot_epoch),
                actual_snapshot_epoch: Some(tx.snapshot_epoch),
                coordinator_snapshot_epoch: Some(coordinator_epoch),
                denied_capability: denied_capability_tag(bound_decision),
            },
            tx.result_kind,
        );
    }
    if matches!(bound_decision, FecCapabilityDecision::Denied(_)) {
        return reject_commit(
            CommitStatus::RejectedCapabilityDenied,
            state.token,
            "capability denied for transaction",
            CommitRejectDetail {
                code: CommitRejectCode::CapabilityDenied,
                expected_token: Some(state.token),
                actual_token: Some(tx.token),
                expected_snapshot_epoch: Some(tx.snapshot_epoch),
                actual_snapshot_epoch: Some(tx.snapshot_epoch),
                coordinator_snapshot_epoch: Some(coordinator_epoch),
                denied_capability: denied_capability_tag(bound_decision),
            },
            tx.result_kind,
        );
    }
    apply_transaction(state, tx, token_nonce)
}

fn format_reject_detail(detail: &CommitRejectDetail) -> String {
    format!(
        "code:{:?}|expected_token:{}|actual_token:{}|expected_snapshot:{}|actual_snapshot:{}|coordinator_snapshot:{}|denied_cap:{}",
        detail.code,
        detail
            .expected_token
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        detail
            .actual_token
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        detail
            .expected_snapshot_epoch
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        detail
            .actual_snapshot_epoch
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        detail
            .coordinator_snapshot_epoch
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        detail
            .denied_capability
            .map(|cap| format!("{cap:?}"))
            .unwrap_or_else(|| "none".to_string()),
    )
}

fn update_perf_counters_from_outcome(counters: &mut FecSeamPerfCounters, outcome: &CommitResult) {
    match outcome.status {
        CommitStatus::Applied => {
            counters.commit_applied_count = counters.commit_applied_count.saturating_add(1);
        }
        _ => {
            counters.commit_rejected_count = counters.commit_rejected_count.saturating_add(1);
        }
    }

    counters.dep_delta_cells_total = counters.dep_delta_cells_total.saturating_add(
        (outcome.topology_delta.dependency_delta.added_cells.len()
            + outcome.topology_delta.dependency_delta.removed_cells.len()) as u64,
    );
    counters.dep_delta_names_total = counters.dep_delta_names_total.saturating_add(
        (outcome.topology_delta.dependency_delta.added_names.len()
            + outcome.topology_delta.dependency_delta.removed_names.len()) as u64,
    );
    counters.dep_delta_spill_children_total =
        counters.dep_delta_spill_children_total.saturating_add(
            (outcome
                .topology_delta
                .dependency_delta
                .added_spill_children
                .len()
                + outcome
                    .topology_delta
                    .dependency_delta
                    .removed_spill_children
                    .len()) as u64,
        );
    if !outcome.topology_delta.is_empty() {
        counters.token_rotation_count = counters.token_rotation_count.saturating_add(1);
    }

    match &outcome.shape_delta.spill_event {
        SpillDeltaEvent::None => {}
        SpillDeltaEvent::SpillTakeover {
            entered_cells,
            exited_cells,
            ..
        } => {
            counters.spill_hint_count = counters.spill_hint_count.saturating_add(1);
            counters.spill_takeover_count = counters.spill_takeover_count.saturating_add(1);
            counters.spill_entered_total = counters
                .spill_entered_total
                .saturating_add(entered_cells.len() as u64);
            counters.spill_exited_total = counters
                .spill_exited_total
                .saturating_add(exited_cells.len() as u64);
        }
        SpillDeltaEvent::SpillClearance { exited_cells, .. } => {
            counters.spill_hint_count = counters.spill_hint_count.saturating_add(1);
            counters.spill_clearance_count = counters.spill_clearance_count.saturating_add(1);
            counters.spill_exited_total = counters
                .spill_exited_total
                .saturating_add(exited_cells.len() as u64);
        }
        SpillDeltaEvent::SpillBlocked { .. } => {
            counters.spill_hint_count = counters.spill_hint_count.saturating_add(1);
            counters.spill_blocked_count = counters.spill_blocked_count.saturating_add(1);
        }
    }

    if let Some(detail) = &outcome.reject_detail {
        match detail.code {
            CommitRejectCode::SessionNotFound => {
                counters.reject_session_not_found_count =
                    counters.reject_session_not_found_count.saturating_add(1);
            }
            CommitRejectCode::FormulaNotRegistered => {
                counters.reject_formula_not_registered_count = counters
                    .reject_formula_not_registered_count
                    .saturating_add(1);
            }
            CommitRejectCode::SessionFormulaMismatch => {
                counters.reject_formula_mismatch_count =
                    counters.reject_formula_mismatch_count.saturating_add(1);
            }
            CommitRejectCode::ExpectedTokenMismatch => {
                counters.reject_expected_token_mismatch_count = counters
                    .reject_expected_token_mismatch_count
                    .saturating_add(1);
            }
            CommitRejectCode::TransactionTokenMismatch => {
                counters.reject_transaction_token_mismatch_count = counters
                    .reject_transaction_token_mismatch_count
                    .saturating_add(1);
            }
            CommitRejectCode::CapabilityNotBound => {
                counters.reject_capability_not_bound_count =
                    counters.reject_capability_not_bound_count.saturating_add(1);
            }
            CommitRejectCode::CapabilityDecisionMismatch => {
                counters.reject_capability_decision_mismatch_count = counters
                    .reject_capability_decision_mismatch_count
                    .saturating_add(1);
            }
            CommitRejectCode::SnapshotMismatch => {
                counters.reject_snapshot_mismatch_count =
                    counters.reject_snapshot_mismatch_count.saturating_add(1);
            }
            CommitRejectCode::CoordinatorSnapshotMismatch => {
                counters.reject_coordinator_snapshot_mismatch_count = counters
                    .reject_coordinator_snapshot_mismatch_count
                    .saturating_add(1);
            }
            CommitRejectCode::CapabilityDenied => {
                counters.reject_capability_denied_count =
                    counters.reject_capability_denied_count.saturating_add(1);
            }
        }
    }
}

fn capability_decision_name(decision: FecCapabilityDecision) -> &'static str {
    match decision {
        FecCapabilityDecision::Allowed => "allowed",
        FecCapabilityDecision::Denied(_) => "denied",
    }
}

fn topology_impact_name(impact: TopologyImpact) -> &'static str {
    match impact {
        TopologyImpact::None => "none",
        TopologyImpact::DependencySetChanged => "dependency_set_changed",
        TopologyImpact::SpillRangeChanged => "spill_range_changed",
        TopologyImpact::SpillBlocked => "spill_blocked",
    }
}

fn apply_transaction(
    state: &mut FormulaRuntimeState,
    tx: &EvalTransaction,
    token_nonce: &mut u64,
) -> CommitResult {
    let observed = observed_dependencies_from_observations(&tx.observations);
    let dependency_delta = dependency_delta(&state.observed_dependencies, &observed);
    let (spill_event, new_spill_range) = spill_event(
        &tx.formula_id,
        state.last_spill_range,
        tx.result_kind,
        &tx.runtime,
    );
    let shape_delta = FecShapeDelta { spill_event };
    let topology_delta = build_topology_delta(dependency_delta, &shape_delta);
    let value = tx.runtime.to_scalar();
    let value_delta = FecValueDelta {
        changed: state.last_scalar_value.as_ref() != Some(&value)
            || state.last_result_kind != Some(tx.result_kind),
        previous_result_kind: state.last_result_kind,
        next_result_kind: tx.result_kind,
    };

    state.observed_dependencies = observed;
    state.last_spill_range = new_spill_range;
    state.last_result_kind = Some(tx.result_kind);
    state.last_scalar_value = Some(value.clone());

    if !topology_delta.is_empty() {
        state.token = bump_token(state.token, token_nonce);
    }

    CommitResult {
        status: CommitStatus::Applied,
        reject_detail: None,
        new_token: state.token,
        value,
        value_delta,
        shape_delta,
        topology_delta,
    }
}

fn reject_commit(
    status: CommitStatus,
    token: FormulaToken,
    message: &str,
    detail: CommitRejectDetail,
    next_result_kind: F3eResultKind,
) -> CommitResult {
    CommitResult {
        status,
        reject_detail: Some(detail),
        new_token: token,
        value: Value::Error(CellError::Ref(message.to_string())),
        value_delta: FecValueDelta {
            changed: false,
            previous_result_kind: None,
            next_result_kind,
        },
        shape_delta: FecShapeDelta::default(),
        topology_delta: FecTopologyDelta::default(),
    }
}

fn observed_dependencies_from_observations(
    observations: &[EvalObservation],
) -> F3eObservedDependencies {
    let mut observed = F3eObservedDependencies::default();
    for observation in observations {
        match observation {
            EvalObservation::ReadCell(cell) => {
                observed.cells_read.insert(*cell);
            }
            EvalObservation::ReadName(name_id) => {
                observed.names_read.insert(*name_id);
            }
            EvalObservation::ReadSpillChild(cell) => {
                observed.spill_children_read.insert(*cell);
            }
            EvalObservation::VolatileRead | EvalObservation::ExternalRead => {}
        }
    }
    observed
}

fn dependency_delta(
    previous: &F3eObservedDependencies,
    next: &F3eObservedDependencies,
) -> F3eDependencyDelta {
    let mut delta = F3eDependencyDelta::default();
    for cell in &next.cells_read {
        if !previous.cells_read.contains(cell) {
            delta.added_cells.insert(*cell);
        }
    }
    for cell in &previous.cells_read {
        if !next.cells_read.contains(cell) {
            delta.removed_cells.insert(*cell);
        }
    }
    for name_id in &next.names_read {
        if !previous.names_read.contains(name_id) {
            delta.added_names.insert(*name_id);
        }
    }
    for name_id in &previous.names_read {
        if !next.names_read.contains(name_id) {
            delta.removed_names.insert(*name_id);
        }
    }
    for cell in &next.spill_children_read {
        if !previous.spill_children_read.contains(cell) {
            delta.added_spill_children.insert(*cell);
        }
    }
    for cell in &previous.spill_children_read {
        if !next.spill_children_read.contains(cell) {
            delta.removed_spill_children.insert(*cell);
        }
    }
    delta
}

fn spill_event(
    formula_id: &FecFormulaId,
    previous_range: Option<CellRange>,
    result_kind: F3eResultKind,
    runtime: &RuntimeValue,
) -> (SpillDeltaEvent, Option<CellRange>) {
    let anchor = match formula_id {
        FecFormulaId::Cell(cell) => *cell,
        FecFormulaId::Name(_) => return (SpillDeltaEvent::None, None),
    };

    let next_range = if matches!(result_kind, F3eResultKind::ArraySpill) {
        runtime.as_array().map(|array| {
            let end = CellRef {
                col: anchor.col + array.cols() as u16 - 1,
                row: anchor.row + array.rows() as u16 - 1,
            };
            CellRange::new(anchor, end)
        })
    } else {
        None
    };

    if matches!(result_kind, F3eResultKind::Error)
        && is_spill_error(runtime)
        && previous_range.is_none()
    {
        return (
            SpillDeltaEvent::SpillBlocked {
                anchor,
                attempted_range_id: None,
                attempted_range: None,
                block_cause: SpillBlockCause::RuntimeError,
                invalidation_scope: SpillInvalidationScope::None,
            },
            None,
        );
    }

    let event = match (previous_range, next_range) {
        (None, None) => SpillDeltaEvent::None,
        (None, Some(new_range)) => {
            let (entered_cells, exited_cells) = spill_cells_delta(None, Some(new_range));
            SpillDeltaEvent::SpillTakeover {
                anchor,
                previous_range_id: None,
                new_range_id: spill_range_id(anchor, new_range),
                old_range: None,
                new_range,
                entered_cells,
                exited_cells,
                invalidation_scope: SpillInvalidationScope::EnteredExitedCells,
            }
        }
        (Some(old_range), None) => {
            let (_, exited_cells) = spill_cells_delta(Some(old_range), None);
            SpillDeltaEvent::SpillClearance {
                anchor,
                cleared_range_id: spill_range_id(anchor, old_range),
                old_range,
                exited_cells,
                invalidation_scope: SpillInvalidationScope::EnteredExitedCells,
            }
        }
        (Some(old_range), Some(new_range)) => {
            if old_range == new_range {
                SpillDeltaEvent::None
            } else {
                let (entered_cells, exited_cells) =
                    spill_cells_delta(Some(old_range), Some(new_range));
                SpillDeltaEvent::SpillTakeover {
                    anchor,
                    previous_range_id: Some(spill_range_id(anchor, old_range)),
                    new_range_id: spill_range_id(anchor, new_range),
                    old_range: Some(old_range),
                    new_range,
                    entered_cells,
                    exited_cells,
                    invalidation_scope: SpillInvalidationScope::PreviousAndCurrentRanges,
                }
            }
        }
    };

    let stored = match event {
        SpillDeltaEvent::None => previous_range,
        SpillDeltaEvent::SpillTakeover { new_range, .. } => Some(new_range),
        SpillDeltaEvent::SpillClearance { .. } | SpillDeltaEvent::SpillBlocked { .. } => None,
    };
    (event, stored)
}

fn spill_cells_delta(
    old_range: Option<CellRange>,
    new_range: Option<CellRange>,
) -> (Vec<CellRef>, Vec<CellRef>) {
    let old_cells: FxHashSet<CellRef> = old_range
        .map(|range| range.iter().collect::<FxHashSet<CellRef>>())
        .unwrap_or_default();
    let new_cells: FxHashSet<CellRef> = new_range
        .map(|range| range.iter().collect::<FxHashSet<CellRef>>())
        .unwrap_or_default();

    let mut entered_cells: Vec<CellRef> = new_cells.difference(&old_cells).copied().collect();
    entered_cells.sort_unstable();
    let mut exited_cells: Vec<CellRef> = old_cells.difference(&new_cells).copied().collect();
    exited_cells.sort_unstable();
    (entered_cells, exited_cells)
}

fn spill_event_counts(shape_delta: &FecShapeDelta) -> (usize, usize) {
    (
        shape_delta.spill_event.entered_cells().len(),
        shape_delta.spill_event.exited_cells().len(),
    )
}

fn build_topology_delta(
    dependency_delta: F3eDependencyDelta,
    shape_delta: &FecShapeDelta,
) -> FecTopologyDelta {
    let mut impacted_cells: FxHashSet<CellRef> = FxHashSet::default();
    let mut impacted_names = FxHashSet::default();
    for cell in &dependency_delta.added_cells {
        impacted_cells.insert(*cell);
    }
    for cell in &dependency_delta.removed_cells {
        impacted_cells.insert(*cell);
    }
    for cell in &dependency_delta.added_spill_children {
        impacted_cells.insert(*cell);
    }
    for cell in &dependency_delta.removed_spill_children {
        impacted_cells.insert(*cell);
    }
    for name_id in &dependency_delta.added_names {
        impacted_names.insert(*name_id);
    }
    for name_id in &dependency_delta.removed_names {
        impacted_names.insert(*name_id);
    }

    let impact = match &shape_delta.spill_event {
        SpillDeltaEvent::None => {
            if dependency_delta.is_empty() {
                TopologyImpact::None
            } else {
                TopologyImpact::DependencySetChanged
            }
        }
        SpillDeltaEvent::SpillTakeover {
            entered_cells,
            exited_cells,
            ..
        } => {
            for cell in entered_cells {
                impacted_cells.insert(*cell);
            }
            for cell in exited_cells {
                impacted_cells.insert(*cell);
            }
            TopologyImpact::SpillRangeChanged
        }
        SpillDeltaEvent::SpillClearance { exited_cells, .. } => {
            for cell in exited_cells {
                impacted_cells.insert(*cell);
            }
            TopologyImpact::SpillRangeChanged
        }
        SpillDeltaEvent::SpillBlocked { .. } => TopologyImpact::SpillBlocked,
    };

    FecTopologyDelta {
        dependency_delta,
        impacted_cells,
        impacted_names,
        impact,
    }
}

fn is_spill_error(runtime: &RuntimeValue) -> bool {
    matches!(
        runtime,
        RuntimeValue::Scalar(Value::Error(CellError::Spill(_)))
    )
}

fn denied_capability_tag(decision: FecCapabilityDecision) -> Option<FecCapabilityTag> {
    match decision {
        FecCapabilityDecision::Allowed => None,
        FecCapabilityDecision::Denied(tag) => Some(tag),
    }
}

fn bump_token(current: FormulaToken, token_nonce: &mut u64) -> FormulaToken {
    *token_nonce = token_nonce.wrapping_add(1);
    let nonce = *token_nonce as u128;
    let mixed = current ^ (nonce << 32) ^ 0x9e37_79b9_7f4a_7c15_u128;
    if mixed == 0 { 1 } else { mixed }
}

fn default_provided_capabilities() -> FxHashSet<FecCapabilityTag> {
    let mut caps = FxHashSet::default();
    caps.insert(FecCapabilityTag::ReferenceResolution);
    caps.insert(FecCapabilityTag::CallerContext);
    caps.insert(FecCapabilityTag::TimeProvider);
    caps.insert(FecCapabilityTag::RandomProvider);
    caps.insert(FecCapabilityTag::ExternalProvider);
    caps.insert(FecCapabilityTag::LocaleParseFormat);
    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn commit_rejects_missing_session_with_structured_code() {
        let mut host = DefaultFecHost::default();
        let tx = EvalTransaction {
            session_id: 999,
            formula_id: FecFormulaId::Cell(CellRef { col: 1, row: 1 }),
            token: 1,
            snapshot_epoch: 7,
            capability_decision: FecCapabilityDecision::Allowed,
            result_kind: F3eResultKind::Scalar,
            runtime: RuntimeValue::scalar(Value::Number(1.0)),
            observations: Vec::new(),
        };
        let result = host.commit(tx);
        assert_eq!(result.status, CommitStatus::RejectedSessionNotFound);
        assert_eq!(
            result.reject_detail.as_ref().map(|detail| detail.code),
            Some(CommitRejectCode::SessionNotFound)
        );
    }

    #[test]
    fn spill_error_runtime_emits_spill_blocked_shape_event() {
        let mut host = DefaultFecHost::default();
        let formula_id = FecFormulaId::Cell(CellRef { col: 1, row: 1 });
        let plan = FormulaPlan {
            token: 11,
            expr: Rc::new(crate::ast::Expr::Number(1.0)),
            static_dependencies: FxHashSet::default(),
            required_capabilities: vec![FecCapabilityTag::ReferenceResolution],
            dependency_profile: F3eDependencyProfile::RefOnly,
        };
        host.install_plan(formula_id, &plan);
        host.set_coordinator_epoch(5);
        let session_id = host.open_session(&formula_id, Some(11), 5);
        host.capability_view(
            session_id,
            &formula_id,
            &[FecCapabilityTag::ReferenceResolution],
        );
        let result = host.commit(EvalTransaction {
            session_id,
            formula_id,
            token: 11,
            snapshot_epoch: 5,
            capability_decision: FecCapabilityDecision::Allowed,
            result_kind: F3eResultKind::Error,
            runtime: RuntimeValue::scalar(Value::Error(CellError::Spill(
                "blocked".to_string(),
            ))),
            observations: Vec::new(),
        });
        assert_eq!(result.status, CommitStatus::Applied);
        assert!(matches!(
            result.shape_delta.spill_event,
            SpillDeltaEvent::SpillBlocked { .. }
        ));
    }
}
