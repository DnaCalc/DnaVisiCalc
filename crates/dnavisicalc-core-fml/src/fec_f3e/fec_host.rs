use rustc_hash::{FxHashMap, FxHashSet};

use crate::address::{CellRange, CellRef};
use crate::eval::{CellError, RuntimeValue, Value};

use super::contracts::{
    CommitResult, CommitStatus, EvalObservation, EvalSessionId, EvalTransaction,
    F3eDependencyDelta, F3eDependencyProfile, F3eObservedDependencies, F3eResultKind,
    FecCapabilityDecision, FecCapabilityTag, FecCapabilityView, FecCoordinator, FecFormulaId,
    FormulaPlan, FormulaToken, SpillShapeDelta,
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
}

#[derive(Debug, Clone)]
struct EvalSessionState {
    formula_id: FecFormulaId,
    expected_token: Option<FormulaToken>,
    snapshot_epoch: u64,
}

#[derive(Debug)]
pub struct DefaultFecHost {
    formulas: FxHashMap<FecFormulaId, FormulaRuntimeState>,
    sessions: FxHashMap<EvalSessionId, EvalSessionState>,
    next_session_id: EvalSessionId,
    token_nonce: u64,
    provided_capabilities: FxHashSet<FecCapabilityTag>,
}

impl Default for DefaultFecHost {
    fn default() -> Self {
        Self {
            formulas: FxHashMap::default(),
            sessions: FxHashMap::default(),
            next_session_id: 0,
            token_nonce: 0,
            provided_capabilities: default_provided_capabilities(),
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
    fn install_plan(&mut self, formula_id: FecFormulaId, plan: &FormulaPlan) -> FormulaToken {
        let trace_start = boundary_trace_start();
        let formula_id_text = format_formula_id(&formula_id);
        let (observed_dependencies, last_spill_range) =
            if let Some(previous) = self.formulas.get(&formula_id) {
                (
                    previous.observed_dependencies.clone(),
                    previous.last_spill_range,
                )
            } else {
                (F3eObservedDependencies::default(), None)
            };
        self.formulas.insert(
            formula_id,
            FormulaRuntimeState {
                token: plan.token,
                required_capabilities: plan.required_capabilities.clone(),
                dependency_profile: plan.dependency_profile,
                observed_dependencies,
                last_spill_range,
            },
        );
        boundary_trace_event(
            "fec.install_plan",
            &[
                ("formula_id", formula_id_text),
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
        &self,
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
        boundary_trace_event(
            "fec.capability_view",
            &[
                ("formula_id", format_formula_id(formula_id)),
                ("required_caps", format_capabilities(required)),
                ("required_caps_count", required.len().to_string()),
                (
                    "provided_caps_count",
                    self.provided_capabilities.len().to_string(),
                ),
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
        self.next_session_id = self.next_session_id.wrapping_add(1);
        let session_id = self.next_session_id;
        self.sessions.insert(
            session_id,
            EvalSessionState {
                formula_id: formula_id.clone(),
                expected_token,
                snapshot_epoch,
            },
        );
        boundary_trace_event(
            "fec.open_session",
            &[
                ("formula_id", format_formula_id(formula_id)),
                ("session_id", session_id.to_string()),
                (
                    "expected_token",
                    expected_token
                        .map(|token| token.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                ("snapshot_epoch", snapshot_epoch.to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        session_id
    }

    fn commit(&mut self, tx: EvalTransaction) -> CommitResult {
        let trace_start = boundary_trace_start();
        let formula_id_text = format_formula_id(&tx.formula_id);
        let outcome = if let Some(session) = self.sessions.remove(&tx.session_id) {
            if session.formula_id != tx.formula_id {
                reject_commit(
                    CommitStatus::RejectedSnapshotConflict,
                    tx.token,
                    "session formula mismatch",
                )
            } else if let Some(state) = self.formulas.get_mut(&tx.formula_id) {
                if let Some(expected) = session.expected_token {
                    if expected != state.token {
                        reject_commit(
                            CommitStatus::RejectedTokenMismatch,
                            state.token,
                            "expected token does not match coordinator token",
                        )
                    } else if tx.token != state.token {
                        reject_commit(
                            CommitStatus::RejectedTokenMismatch,
                            state.token,
                            "transaction token mismatch",
                        )
                    } else if tx.snapshot_epoch != session.snapshot_epoch {
                        reject_commit(
                            CommitStatus::RejectedSnapshotConflict,
                            state.token,
                            "snapshot epoch mismatch",
                        )
                    } else if matches!(tx.capability_decision, FecCapabilityDecision::Denied(_)) {
                        reject_commit(
                            CommitStatus::RejectedCapabilityDenied,
                            state.token,
                            "capability denied for transaction",
                        )
                    } else {
                        apply_transaction(state, &tx, &mut self.token_nonce)
                    }
                } else if tx.snapshot_epoch != session.snapshot_epoch {
                    reject_commit(
                        CommitStatus::RejectedSnapshotConflict,
                        state.token,
                        "snapshot epoch mismatch",
                    )
                } else if matches!(tx.capability_decision, FecCapabilityDecision::Denied(_)) {
                    reject_commit(
                        CommitStatus::RejectedCapabilityDenied,
                        state.token,
                        "capability denied for transaction",
                    )
                } else {
                    apply_transaction(state, &tx, &mut self.token_nonce)
                }
            } else {
                reject_commit(
                    CommitStatus::RejectedTokenMismatch,
                    tx.token,
                    "formula plan is not registered",
                )
            }
        } else {
            reject_commit(
                CommitStatus::RejectedSnapshotConflict,
                tx.token,
                "session not found for commit",
            )
        };

        boundary_trace_event(
            "fec.commit",
            &[
                ("formula_id", formula_id_text),
                ("session_id", tx.session_id.to_string()),
                ("token", tx.token.to_string()),
                ("result_kind", result_kind_name(tx.result_kind).to_string()),
                ("status", format!("{:?}", outcome.status)),
                ("new_token", outcome.new_token.to_string()),
                (
                    "dep_delta_cells",
                    (outcome.dependency_delta.added_cells.len()
                        + outcome.dependency_delta.removed_cells.len())
                    .to_string(),
                ),
                (
                    "dep_delta_names",
                    (outcome.dependency_delta.added_names.len()
                        + outcome.dependency_delta.removed_names.len())
                    .to_string(),
                ),
                (
                    "spill_shape_delta",
                    spill_shape_name(&outcome.spill_shape_delta).to_string(),
                ),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        outcome
    }
}

fn apply_transaction(
    state: &mut FormulaRuntimeState,
    tx: &EvalTransaction,
    token_nonce: &mut u64,
) -> CommitResult {
    let observed = observed_dependencies_from_observations(&tx.observations);
    let dependency_delta = dependency_delta(&state.observed_dependencies, &observed);
    let (spill_shape_delta, new_spill_range) = spill_shape_delta(
        &tx.formula_id,
        state.last_spill_range,
        tx.result_kind,
        &tx.runtime,
    );
    let value = tx.runtime.to_scalar();

    state.observed_dependencies = observed;
    state.last_spill_range = new_spill_range;

    if !dependency_delta.is_empty() || !matches!(spill_shape_delta, SpillShapeDelta::None) {
        state.token = bump_token(state.token, token_nonce);
    }

    CommitResult {
        status: CommitStatus::Applied,
        new_token: state.token,
        dependency_delta,
        spill_shape_delta,
        value,
    }
}

fn reject_commit(status: CommitStatus, token: FormulaToken, message: &str) -> CommitResult {
    CommitResult {
        status,
        new_token: token,
        dependency_delta: F3eDependencyDelta::default(),
        spill_shape_delta: SpillShapeDelta::None,
        value: Value::Error(CellError::Ref(message.to_string())),
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
            EvalObservation::ReadName(name) => {
                observed.names_read.insert(name.clone());
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

    for name in &next.names_read {
        if !previous.names_read.contains(name) {
            delta.added_names.insert(name.clone());
        }
    }
    for name in &previous.names_read {
        if !next.names_read.contains(name) {
            delta.removed_names.insert(name.clone());
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

fn spill_shape_delta(
    formula_id: &FecFormulaId,
    previous_range: Option<CellRange>,
    result_kind: F3eResultKind,
    runtime: &RuntimeValue,
) -> (SpillShapeDelta, Option<CellRange>) {
    let anchor = match formula_id {
        FecFormulaId::Cell(cell) => *cell,
        FecFormulaId::Name(_) => return (SpillShapeDelta::None, None),
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

    let delta = match (previous_range, next_range) {
        (None, None) => SpillShapeDelta::None,
        (None, Some(new_range)) => SpillShapeDelta::Created {
            anchor,
            range: new_range,
        },
        (Some(old_range), None) => SpillShapeDelta::Cleared {
            anchor,
            old: old_range,
        },
        (Some(old_range), Some(new_range)) => {
            if old_range == new_range {
                SpillShapeDelta::None
            } else {
                SpillShapeDelta::Resized {
                    anchor,
                    old: old_range,
                    new: new_range,
                }
            }
        }
    };

    let stored = match delta {
        SpillShapeDelta::Cleared { .. } => None,
        _ => next_range,
    };
    (delta, stored)
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
    fn exposes_interface_version_marker() {
        let host = DefaultFecHost::default();
        assert_eq!(host.interface_version(), FEC_F3E_INTERFACE_VERSION);
    }

    #[test]
    fn tracks_plan_tokens() {
        let mut host = DefaultFecHost::default();
        let id = FecFormulaId::Cell(CellRef { col: 1, row: 1 });
        let plan = FormulaPlan {
            token: 11,
            expr: Rc::new(crate::ast::Expr::Number(1.0)),
            static_dependencies: FxHashSet::default(),
            required_capabilities: vec![FecCapabilityTag::ReferenceResolution],
            dependency_profile: F3eDependencyProfile::RefOnly,
        };
        let token = host.install_plan(id.clone(), &plan);
        assert_eq!(token, 11);
        assert_eq!(host.registration_token_for(&id), Some(11));
    }
}
