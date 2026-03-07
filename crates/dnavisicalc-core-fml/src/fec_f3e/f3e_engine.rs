use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::address::SheetBounds;
use crate::ast::Expr;
use crate::deps::dependencies_for_expr;
use crate::eval::EvalContext;
use crate::parser::parse_formula;

use super::contracts::{
    F3eCompiledFormula, F3eDeclaredDependencies, F3eDependencyDeclContext, F3eDependencyProfile,
    F3eEngine, F3eEvalContext, F3eEvalResult, F3eEvalTarget, FecCapabilityTag,
};
use super::trace::{
    boundary_duration_us, boundary_trace_event, boundary_trace_start, format_capabilities,
    format_eval_target, runtime_result_kind,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CoreF3eEngine;

impl CoreF3eEngine {
    pub fn compile_bound_expr(&self, expr: Rc<Expr>) -> F3eCompiledFormula {
        let static_dependencies = dependencies_for_expr(&expr);
        let required_capabilities = required_capabilities_for_expr(&expr);
        let dependency_profile = classify_dependency_profile(&required_capabilities);
        F3eCompiledFormula {
            expr,
            static_dependencies,
            required_capabilities,
            dependency_profile,
        }
    }
}

impl F3eEngine for CoreF3eEngine {
    fn compile(
        &self,
        formula_text: &str,
        bounds: SheetBounds,
        _ctx: &super::contracts::F3eCompileContext,
    ) -> Result<F3eCompiledFormula, crate::ParseError> {
        let trace_start = boundary_trace_start();
        let expr = match parse_formula(formula_text, bounds) {
            Ok(expr) => expr,
            Err(err) => {
                boundary_trace_event(
                    "f3e.compile",
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
        let compiled = self.compile_bound_expr(Rc::new(expr));
        boundary_trace_event(
            "f3e.compile",
            &[
                ("dep_count", compiled.static_dependencies.len().to_string()),
                (
                    "required_caps",
                    format_capabilities(&compiled.required_capabilities),
                ),
                (
                    "dependency_profile",
                    format!("{:?}", compiled.dependency_profile),
                ),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        Ok(compiled)
    }

    fn declare_dependencies(
        &self,
        compiled: &F3eCompiledFormula,
        ctx: &F3eDependencyDeclContext,
    ) -> F3eDeclaredDependencies {
        let trace_start = boundary_trace_start();
        let declared = F3eDeclaredDependencies {
            static_dependencies: compiled.static_dependencies.clone(),
            required_capabilities: compiled.required_capabilities.clone(),
            dependency_profile: compiled.dependency_profile,
        };
        boundary_trace_event(
            "f3e.declare_dependencies",
            &[
                ("dep_count", declared.static_dependencies.len().to_string()),
                (
                    "required_caps",
                    format_capabilities(&declared.required_capabilities),
                ),
                (
                    "dependency_profile",
                    format!("{:?}", declared.dependency_profile),
                ),
                (
                    "prior_token",
                    ctx.prior_token
                        .map(|token| token.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        declared
    }

    fn evaluate(
        &self,
        evaluator: &mut EvalContext<'_>,
        target: F3eEvalTarget<'_>,
        ctx: &F3eEvalContext,
    ) -> F3eEvalResult {
        let trace_start = boundary_trace_start();
        let target_text = format_eval_target(&target);
        // TODO(FEC/F3E): enforce capability denials with deterministic error
        // mapping once profile gating is finalized.
        let capability_contract_satisfied = ctx.capabilities.supports_required();
        let runtime = match &target {
            F3eEvalTarget::Cell(cell) => evaluator.evaluate_cell_runtime(*cell),
            F3eEvalTarget::Name(name) => evaluator.evaluate_name_runtime(name),
        };
        boundary_trace_event(
            "f3e.evaluate",
            &[
                ("target", target_text),
                (
                    "required_caps",
                    format_capabilities(ctx.capabilities.required_capabilities()),
                ),
                (
                    "required_caps_count",
                    ctx.capabilities.required_capabilities().len().to_string(),
                ),
                (
                    "provided_caps_count",
                    ctx.capabilities.provided_capability_count().to_string(),
                ),
                (
                    "supports_required",
                    capability_contract_satisfied.to_string(),
                ),
                ("result_kind", runtime_result_kind(&runtime).to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        F3eEvalResult { runtime }
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
        let compiled = engine.compile_bound_expr(Rc::new(Expr::Number(1.0)));
        assert_eq!(compiled.dependency_profile, F3eDependencyProfile::None);
    }
}
