use std::hash::Hasher;
use std::rc::Rc;

use rustc_hash::{FxHashSet, FxHasher};

use crate::address::{CellRef, SheetBounds};
use crate::ast::Expr;
use crate::deps::dependencies_for_expr;
use crate::eval::{CellError, EvalContext, ObservedAccesses, Value};
use crate::parser::parse_formula;

use super::contracts::{
    EvalObservation, EvalRequest, EvalTransaction, F3eDependencyProfile, F3eKernel,
    F3ePrepareContext, F3eResultKind, FecCapabilityDecision, FecCapabilityTag, FormulaPlan,
    FormulaToken,
};
use super::trace::{
    boundary_duration_us, boundary_trace_event, boundary_trace_start, format_capabilities,
    format_eval_target, format_formula_id, result_kind_name,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CoreF3eEngine;

impl CoreF3eEngine {
    pub fn prepare_bound_expr(&self, formula_text: &str, expr: Rc<Expr>) -> FormulaPlan {
        let static_dependencies = dependencies_for_expr(&expr);
        let required_capabilities = required_capabilities_for_expr(&expr);
        let dependency_profile = classify_dependency_profile(&required_capabilities);
        let token = compute_formula_token(
            formula_text,
            &static_dependencies,
            &required_capabilities,
            dependency_profile,
        );
        FormulaPlan {
            token,
            expr,
            static_dependencies,
            required_capabilities,
            dependency_profile,
        }
    }
}

impl F3eKernel for CoreF3eEngine {
    fn prepare(
        &self,
        formula_text: &str,
        bounds: SheetBounds,
        _ctx: &F3ePrepareContext,
    ) -> Result<FormulaPlan, crate::ParseError> {
        let trace_start = boundary_trace_start();
        let expr = match parse_formula(formula_text, bounds) {
            Ok(expr) => expr,
            Err(err) => {
                boundary_trace_event(
                    "f3e.prepare",
                    &[
                        ("dep_count", "0".to_string()),
                        ("required_caps", "none".to_string()),
                        ("dependency_profile", "parse_error".to_string()),
                        ("duration_us", boundary_duration_us(trace_start).to_string()),
                    ],
                );
                return Err(err);
            }
        };
        let plan = self.prepare_bound_expr(formula_text, Rc::new(expr));
        boundary_trace_event(
            "f3e.prepare",
            &[
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
        Ok(plan)
    }

    fn execute(&self, evaluator: &mut EvalContext<'_>, req: EvalRequest<'_>) -> EvalTransaction {
        let trace_start = boundary_trace_start();
        let target_text = format_eval_target(&req.target);
        let capability_decision = req.capability_view.decision;
        let runtime = match capability_decision {
            FecCapabilityDecision::Allowed => match &req.target {
                super::contracts::F3eEvalTarget::Cell(cell) => {
                    evaluator.evaluate_cell_runtime(*cell)
                }
                super::contracts::F3eEvalTarget::Name { label, .. } => {
                    evaluator.evaluate_name_runtime(label)
                }
            },
            FecCapabilityDecision::Denied(tag) => crate::eval::RuntimeValue::scalar(Value::Error(
                CellError::Ref(format!("capability denied: {tag:?}")),
            )),
        };
        let observed_accesses = evaluator.take_observed_accesses();
        let observations = observations_from_accesses(observed_accesses);
        let result_kind = runtime_result_kind(&runtime);
        boundary_trace_event(
            "f3e.execute",
            &[
                ("formula_id", format_formula_id(&req.formula_id)),
                ("target", target_text),
                ("session_id", req.session_id.to_string()),
                ("token", req.token.to_string()),
                (
                    "required_caps",
                    format_capabilities(req.capability_view.required_capabilities()),
                ),
                (
                    "supports_required",
                    req.capability_view.supports_required().to_string(),
                ),
                (
                    "decision",
                    match capability_decision {
                        FecCapabilityDecision::Allowed => "allowed".to_string(),
                        FecCapabilityDecision::Denied(tag) => format!("denied:{tag:?}"),
                    },
                ),
                ("result_kind", result_kind_name(result_kind).to_string()),
                ("observation_count", observations.len().to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        EvalTransaction {
            session_id: req.session_id,
            formula_id: req.formula_id,
            token: req.token,
            snapshot_epoch: req.snapshot_epoch,
            capability_decision,
            result_kind,
            runtime,
            observations,
        }
    }
}

fn observations_from_accesses(mut accesses: ObservedAccesses) -> Vec<EvalObservation> {
    let mut observations: Vec<EvalObservation> = Vec::new();
    let mut cells: Vec<CellRef> = accesses.cells.drain().collect();
    cells.sort();
    for cell in cells {
        observations.push(EvalObservation::ReadCell(cell));
    }

    let mut names: Vec<String> = accesses.names.drain().collect();
    names.sort();
    for name in names {
        observations.push(EvalObservation::ReadName(
            super::contracts::FecNameId::from_canonical_name(&name),
        ));
    }

    let mut spill_children: Vec<CellRef> = accesses.spill_children.drain().collect();
    spill_children.sort();
    for cell in spill_children {
        observations.push(EvalObservation::ReadSpillChild(cell));
    }

    if accesses.volatile_read {
        observations.push(EvalObservation::VolatileRead);
    }
    if accesses.external_read {
        observations.push(EvalObservation::ExternalRead);
    }

    observations
}

fn runtime_result_kind(runtime: &crate::eval::RuntimeValue) -> F3eResultKind {
    match runtime {
        crate::eval::RuntimeValue::Scalar(Value::Error(_)) => F3eResultKind::Error,
        crate::eval::RuntimeValue::Scalar(_) => F3eResultKind::Scalar,
        crate::eval::RuntimeValue::Array(array) => {
            if array.is_spill() {
                F3eResultKind::ArraySpill
            } else {
                F3eResultKind::Array
            }
        }
        crate::eval::RuntimeValue::Lambda(_) => F3eResultKind::Lambda,
    }
}

fn compute_formula_token(
    formula_text: &str,
    static_dependencies: &FxHashSet<CellRef>,
    required_capabilities: &[FecCapabilityTag],
    profile: F3eDependencyProfile,
) -> FormulaToken {
    let mut deps: Vec<CellRef> = static_dependencies.iter().copied().collect();
    deps.sort();

    let mut caps = required_capabilities.to_vec();
    caps.sort();

    let mut lo_hasher = FxHasher::default();
    lo_hasher.write(formula_text.as_bytes());
    for dep in &deps {
        lo_hasher.write_u16(dep.col);
        lo_hasher.write_u16(dep.row);
    }
    for cap in &caps {
        lo_hasher.write_u8(capability_code(*cap));
    }
    lo_hasher.write_u8(profile_code(profile));
    let lo = lo_hasher.finish();

    let mut hi_hasher = FxHasher::default();
    hi_hasher.write_u64(lo ^ 0x9e37_79b9_7f4a_7c15);
    hi_hasher.write(formula_text.as_bytes());
    hi_hasher.write_u8(profile_code(profile));
    hi_hasher.write_usize(deps.len());
    hi_hasher.write_usize(caps.len());
    let hi = hi_hasher.finish();

    ((hi as u128) << 64) | lo as u128
}

fn capability_code(tag: FecCapabilityTag) -> u8 {
    match tag {
        FecCapabilityTag::ReferenceResolution => 1,
        FecCapabilityTag::CallerContext => 2,
        FecCapabilityTag::TimeProvider => 3,
        FecCapabilityTag::RandomProvider => 4,
        FecCapabilityTag::ExternalProvider => 5,
        FecCapabilityTag::LocaleParseFormat => 6,
        FecCapabilityTag::FeatureGate => 7,
        FecCapabilityTag::ErrorDetailEnrichment => 8,
    }
}

fn profile_code(profile: F3eDependencyProfile) -> u8 {
    match profile {
        F3eDependencyProfile::None => 1,
        F3eDependencyProfile::RefOnly => 2,
        F3eDependencyProfile::CallerContext => 3,
        F3eDependencyProfile::TimeProvider => 4,
        F3eDependencyProfile::RandomProvider => 5,
        F3eDependencyProfile::ExternalProvider => 6,
        F3eDependencyProfile::LocaleProfile => 7,
        F3eDependencyProfile::Composite => 8,
    }
}

fn required_capabilities_for_expr(expr: &Expr) -> Vec<FecCapabilityTag> {
    let mut out = FxHashSet::default();
    collect_required_capabilities(expr, &mut out);
    let mut ordered: Vec<FecCapabilityTag> = out.into_iter().collect();
    ordered.sort();
    ordered
}

fn collect_required_capabilities(expr: &Expr, out: &mut FxHashSet<FecCapabilityTag>) {
    match expr {
        Expr::Cell(_, _) | Expr::Range(_, _, _) | Expr::SpillRef(_) => {
            out.insert(FecCapabilityTag::ReferenceResolution);
        }
        Expr::Unary { expr, .. } => collect_required_capabilities(expr, out),
        Expr::Binary { left, right, .. } => {
            collect_required_capabilities(left, out);
            collect_required_capabilities(right, out);
        }
        Expr::FunctionCall { name, args } => {
            match name.to_ascii_uppercase().as_str() {
                "ROW" | "COLUMN" => {
                    out.insert(FecCapabilityTag::ReferenceResolution);
                    out.insert(FecCapabilityTag::CallerContext);
                }
                "NOW" => {
                    out.insert(FecCapabilityTag::TimeProvider);
                }
                "RAND" | "RANDARRAY" => {
                    out.insert(FecCapabilityTag::RandomProvider);
                }
                "STREAM" => {
                    out.insert(FecCapabilityTag::ExternalProvider);
                }
                "INDIRECT" | "OFFSET" => {
                    out.insert(FecCapabilityTag::ReferenceResolution);
                }
                _ => {}
            }
            for arg in args {
                collect_required_capabilities(arg, out);
            }
        }
        Expr::Invoke { callee, args } => {
            collect_required_capabilities(callee, out);
            for arg in args {
                collect_required_capabilities(arg, out);
            }
        }
        Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Name(_) => {}
    }
}

fn classify_dependency_profile(required: &[FecCapabilityTag]) -> F3eDependencyProfile {
    use F3eDependencyProfile as Profile;
    if required.is_empty() {
        return Profile::None;
    }
    if required.len() == 1 {
        return match required[0] {
            FecCapabilityTag::ReferenceResolution => Profile::RefOnly,
            FecCapabilityTag::CallerContext => Profile::CallerContext,
            FecCapabilityTag::TimeProvider => Profile::TimeProvider,
            FecCapabilityTag::RandomProvider => Profile::RandomProvider,
            FecCapabilityTag::ExternalProvider => Profile::ExternalProvider,
            FecCapabilityTag::LocaleParseFormat => Profile::LocaleProfile,
            FecCapabilityTag::FeatureGate | FecCapabilityTag::ErrorDetailEnrichment => {
                Profile::Composite
            }
        };
    }
    if required
        .iter()
        .all(|tag| *tag == FecCapabilityTag::ReferenceResolution)
    {
        return Profile::RefOnly;
    }
    Profile::Composite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_none_for_constant_expr() {
        let engine = CoreF3eEngine;
        let plan = engine.prepare_bound_expr("1", Rc::new(Expr::Number(1.0)));
        assert_eq!(plan.dependency_profile, F3eDependencyProfile::None);
    }
}
