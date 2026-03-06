use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::address::{CellRef, SheetBounds};
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

pub type DependencyToken = u64;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct F3eCompileContext {
    pub profile_id: &'static str,
    pub compatibility_version: &'static str,
    pub locale_profile: &'static str,
    pub feature_gate_profile: &'static str,
}

impl Default for F3eCompileContext {
    fn default() -> Self {
        Self {
            profile_id: "FEC-MIN-B",
            compatibility_version: "dvc-v0",
            locale_profile: "en-US-invariant",
            feature_gate_profile: "dvc-v0-default",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct F3eDependencyDeclContext {
    pub prior_token: Option<DependencyToken>,
}

#[derive(Debug, Clone)]
pub struct F3eCompiledFormula {
    pub expr: Rc<Expr>,
    pub static_dependencies: FxHashSet<CellRef>,
    pub required_capabilities: Vec<FecCapabilityTag>,
    pub dependency_profile: F3eDependencyProfile,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct F3eDeclaredDependencies {
    pub static_dependencies: FxHashSet<CellRef>,
    pub required_capabilities: Vec<FecCapabilityTag>,
    pub dependency_profile: F3eDependencyProfile,
}

#[derive(Debug, Clone)]
pub enum F3eEvalTarget<'a> {
    Cell(CellRef),
    Name(&'a str),
}

#[derive(Debug, Clone)]
pub struct ScopedCapabilityView {
    required: Vec<FecCapabilityTag>,
    provided: FxHashSet<FecCapabilityTag>,
}

impl ScopedCapabilityView {
    pub fn new(required: Vec<FecCapabilityTag>, provided: FxHashSet<FecCapabilityTag>) -> Self {
        Self { required, provided }
    }

    pub fn supports(&self, tag: FecCapabilityTag) -> bool {
        self.provided.contains(&tag)
    }

    pub fn supports_required(&self) -> bool {
        self.required.iter().all(|tag| self.supports(*tag))
    }
}

#[derive(Debug, Clone)]
pub struct F3eEvalContext {
    pub capabilities: ScopedCapabilityView,
}

#[derive(Debug, Clone)]
pub struct F3eEvalResult {
    pub runtime: RuntimeValue,
}

#[derive(Debug, Clone)]
pub struct FecPublishedResult {
    pub value: Value,
}

/// SPEC: FEC-F3E-CALL-001
/// Explicit semantic calls exposed by F3E.
pub trait F3eEngine {
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
    ) -> F3eEvalResult;
}

/// SPEC: FEC-F3E-CAP-001, FEC-F3E-DEP-001, FEC-F3E-PUB-001
/// Host responsibilities exposed by FEC.
pub trait FecHost {
    fn capability_view(&self, required: &[FecCapabilityTag]) -> ScopedCapabilityView;

    fn register_dependencies(
        &mut self,
        formula_id: FecFormulaId,
        deps: &F3eDeclaredDependencies,
    ) -> DependencyToken;

    fn publish_result(
        &self,
        formula_id: &FecFormulaId,
        result: &F3eEvalResult,
    ) -> FecPublishedResult;
}
