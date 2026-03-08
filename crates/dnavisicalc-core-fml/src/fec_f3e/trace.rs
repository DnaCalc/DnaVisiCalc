use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use super::contracts::{
    F3eEvalTarget, F3eResultKind, FecCapabilityTag, FecFormulaId, SpillShapeDelta,
};

static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
static TRACE_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn boundary_trace_enabled() -> bool {
    *TRACE_ENABLED.get_or_init(|| {
        std::env::var("DNAVISICALC_FEC_F3E_TRACE")
            .map(|value| value.trim() == "1")
            .unwrap_or(false)
    })
}

pub fn boundary_trace_start() -> Option<Instant> {
    boundary_trace_enabled().then(Instant::now)
}

pub fn boundary_duration_us(start: Option<Instant>) -> u128 {
    start.map(|begin| begin.elapsed().as_micros()).unwrap_or(0)
}

pub fn boundary_trace_event(event: &str, fields: &[(&str, String)]) {
    if !boundary_trace_enabled() {
        return;
    }
    let seq = TRACE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let mut line = format!("fec_f3e seq={seq} event={event}");
    for (key, value) in fields {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(&sanitize_trace_value(value));
    }
    eprintln!("{line}");
}

pub fn format_formula_id(formula_id: &FecFormulaId) -> String {
    match formula_id {
        FecFormulaId::Cell(cell) => format!("cell:{cell}"),
        FecFormulaId::Name(name) => format!("name:{name}"),
    }
}

pub fn format_eval_target(target: &F3eEvalTarget<'_>) -> String {
    match target {
        F3eEvalTarget::Cell(cell) => format!("cell:{cell}"),
        F3eEvalTarget::Name(name) => format!("name:{name}"),
    }
}

pub fn format_capabilities(capabilities: &[FecCapabilityTag]) -> String {
    if capabilities.is_empty() {
        return "none".to_string();
    }
    let mut names: Vec<&'static str> = capabilities.iter().map(capability_tag_name).collect();
    names.sort_unstable();
    names.join("|")
}

pub fn result_kind_name(kind: F3eResultKind) -> &'static str {
    match kind {
        F3eResultKind::Scalar => "scalar",
        F3eResultKind::Array => "array",
        F3eResultKind::ArraySpill => "array_spill",
        F3eResultKind::Error => "error",
        F3eResultKind::Lambda => "lambda",
    }
}

pub fn spill_shape_name(delta: &SpillShapeDelta) -> &'static str {
    match delta {
        SpillShapeDelta::None => "none",
        SpillShapeDelta::Created { .. } => "created",
        SpillShapeDelta::Resized { .. } => "resized",
        SpillShapeDelta::Cleared { .. } => "cleared",
    }
}

fn capability_tag_name(tag: &FecCapabilityTag) -> &'static str {
    match tag {
        FecCapabilityTag::ReferenceResolution => "reference_resolution",
        FecCapabilityTag::CallerContext => "caller_context",
        FecCapabilityTag::TimeProvider => "time_provider",
        FecCapabilityTag::RandomProvider => "random_provider",
        FecCapabilityTag::ExternalProvider => "external_provider",
        FecCapabilityTag::LocaleParseFormat => "locale_parse_format",
        FecCapabilityTag::FeatureGate => "feature_gate",
        FecCapabilityTag::ErrorDetailEnrichment => "error_detail_enrichment",
    }
}

fn sanitize_trace_value(input: &str) -> String {
    if input.is_empty() {
        return "none".to_string();
    }
    input
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}
