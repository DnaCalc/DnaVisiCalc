use rustc_hash::{FxHashMap, FxHashSet};

use super::contracts::{
    DependencyToken, F3eDeclaredDependencies, F3eDependencyProfile, F3eEvalResult,
    FecCapabilityTag, FecFormulaId, FecHost, FecPublishedResult, ScopedCapabilityView,
};
use super::spec::FEC_F3E_INTERFACE_VERSION;
use super::trace::{
    boundary_duration_us, boundary_trace_event, boundary_trace_start, format_capabilities,
    format_formula_id, runtime_result_kind,
};

#[derive(Debug, Clone)]
struct FecDependencyRegistration {
    token: DependencyToken,
    required_capabilities: Vec<FecCapabilityTag>,
    dependency_profile: F3eDependencyProfile,
}

#[derive(Debug)]
pub struct DefaultFecHost {
    registrations: FxHashMap<FecFormulaId, FecDependencyRegistration>,
    next_token: DependencyToken,
    provided_capabilities: FxHashSet<FecCapabilityTag>,
}

impl Default for DefaultFecHost {
    fn default() -> Self {
        Self {
            registrations: FxHashMap::default(),
            next_token: 0,
            provided_capabilities: default_provided_capabilities(),
        }
    }
}

impl DefaultFecHost {
    pub fn unregister_formula(&mut self, formula_id: &FecFormulaId) {
        self.registrations.remove(formula_id);
    }

    pub fn clear(&mut self) {
        self.registrations.clear();
        self.next_token = 0;
    }

    pub fn required_capabilities_for(&self, formula_id: &FecFormulaId) -> &[FecCapabilityTag] {
        if let Some(reg) = self.registrations.get(formula_id) {
            reg.required_capabilities.as_slice()
        } else {
            &[]
        }
    }

    #[allow(dead_code)]
    pub fn interface_version(&self) -> &'static str {
        FEC_F3E_INTERFACE_VERSION
    }

    #[allow(dead_code)]
    pub fn dependency_profile_for(
        &self,
        formula_id: &FecFormulaId,
    ) -> Option<F3eDependencyProfile> {
        self.registrations
            .get(formula_id)
            .map(|reg| reg.dependency_profile)
    }

    #[allow(dead_code)]
    pub fn registration_token_for(&self, formula_id: &FecFormulaId) -> Option<DependencyToken> {
        self.registrations.get(formula_id).map(|reg| reg.token)
    }
}

impl FecHost for DefaultFecHost {
    fn capability_view(&self, required: &[FecCapabilityTag]) -> ScopedCapabilityView {
        let trace_start = boundary_trace_start();
        let view = ScopedCapabilityView::new(required.to_vec(), self.provided_capabilities.clone());
        boundary_trace_event(
            "fec.capability_view",
            &[
                ("required_caps", format_capabilities(required)),
                ("required_caps_count", required.len().to_string()),
                (
                    "provided_caps_count",
                    self.provided_capabilities.len().to_string(),
                ),
                ("supports_required", view.supports_required().to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        view
    }

    fn register_dependencies(
        &mut self,
        formula_id: FecFormulaId,
        deps: &F3eDeclaredDependencies,
    ) -> DependencyToken {
        let trace_start = boundary_trace_start();
        self.next_token = self.next_token.wrapping_add(1);
        let token = self.next_token;
        let formula_id_text = format_formula_id(&formula_id);
        self.registrations.insert(
            formula_id,
            FecDependencyRegistration {
                token,
                required_capabilities: deps.required_capabilities.clone(),
                dependency_profile: deps.dependency_profile,
            },
        );
        boundary_trace_event(
            "fec.register_dependencies",
            &[
                ("formula_id", formula_id_text),
                ("dep_count", deps.static_dependencies.len().to_string()),
                (
                    "required_caps",
                    format_capabilities(&deps.required_capabilities),
                ),
                (
                    "dependency_profile",
                    format!("{:?}", deps.dependency_profile),
                ),
                ("token", token.to_string()),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        token
    }

    fn publish_result(
        &self,
        formula_id: &FecFormulaId,
        result: &F3eEvalResult,
    ) -> FecPublishedResult {
        let trace_start = boundary_trace_start();
        // TODO(FEC/F3E): route format overlays and extended-value metadata once
        // profile contracts are finalized.
        let published = FecPublishedResult {
            value: result.runtime.to_scalar(),
        };
        boundary_trace_event(
            "fec.publish_result",
            &[
                ("formula_id", format_formula_id(formula_id)),
                (
                    "result_kind",
                    runtime_result_kind(&result.runtime).to_string(),
                ),
                ("duration_us", boundary_duration_us(trace_start).to_string()),
            ],
        );
        published
    }
}

fn default_provided_capabilities() -> FxHashSet<FecCapabilityTag> {
    let mut caps = FxHashSet::default();
    caps.insert(FecCapabilityTag::ReferenceResolution);
    caps.insert(FecCapabilityTag::CallerContext);
    caps.insert(FecCapabilityTag::TimeProvider);
    caps.insert(FecCapabilityTag::RandomProvider);
    caps.insert(FecCapabilityTag::ExternalProvider);
    caps.insert(FecCapabilityTag::LocaleParseFormat);
    // TODO(FEC/F3E): split feature-gate lanes by profile/version instead of
    // sharing one monolithic host capability.
    // TODO(FEC/F3E): wire error-detail enrichment once richer diagnostics are
    // represented in the value/result envelope.
    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::CellRef;

    #[test]
    fn exposes_interface_version_marker() {
        let host = DefaultFecHost::default();
        assert_eq!(host.interface_version(), FEC_F3E_INTERFACE_VERSION);
    }

    #[test]
    fn tracks_dependency_registration_tokens() {
        let mut host = DefaultFecHost::default();
        let id = FecFormulaId::Cell(CellRef { col: 1, row: 1 });
        let deps = F3eDeclaredDependencies {
            static_dependencies: FxHashSet::default(),
            required_capabilities: vec![FecCapabilityTag::ReferenceResolution],
            dependency_profile: F3eDependencyProfile::RefOnly,
        };
        let token = host.register_dependencies(id.clone(), &deps);
        assert_eq!(host.registration_token_for(&id), Some(token));
    }
}
