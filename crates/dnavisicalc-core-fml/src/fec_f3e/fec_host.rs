use rustc_hash::{FxHashMap, FxHashSet};

use super::contracts::{
    DependencyToken, F3eDeclaredDependencies, F3eDependencyProfile, F3eEvalResult,
    FecCapabilityTag, FecFormulaId, FecHost, FecPublishedResult, ScopedCapabilityView,
};
use super::spec::FEC_F3E_INTERFACE_VERSION;

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
        ScopedCapabilityView::new(required.to_vec(), self.provided_capabilities.clone())
    }

    fn register_dependencies(
        &mut self,
        formula_id: FecFormulaId,
        deps: &F3eDeclaredDependencies,
    ) -> DependencyToken {
        self.next_token = self.next_token.wrapping_add(1);
        let token = self.next_token;
        self.registrations.insert(
            formula_id,
            FecDependencyRegistration {
                token,
                required_capabilities: deps.required_capabilities.clone(),
                dependency_profile: deps.dependency_profile,
            },
        );
        token
    }

    fn publish_result(
        &self,
        _formula_id: &FecFormulaId,
        result: &F3eEvalResult,
    ) -> FecPublishedResult {
        // TODO(FEC/F3E): route format overlays and extended-value metadata once
        // profile contracts are finalized.
        FecPublishedResult {
            value: result.runtime.to_scalar(),
        }
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
