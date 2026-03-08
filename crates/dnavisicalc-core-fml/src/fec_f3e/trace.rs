use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use super::contracts::{
    F3eEvalTarget, F3eResultKind, FecCapabilityTag, FecFormulaId, FecShapeDelta, SpillDeltaEvent,
};

static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
static TRACE_SEQ: AtomicU64 = AtomicU64::new(0);
pub const FEC_F3E_TRACE_SCHEMA_VERSION: &str = "fec-f3e-trace/b4";

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
    let schema_valid = validate_trace_fields(fields);
    let mut line = format!(
        "fec_f3e trace_version={} seq={seq} event={} schema_valid={}",
        FEC_F3E_TRACE_SCHEMA_VERSION, event, schema_valid
    );
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
        FecFormulaId::Name(name_id) => format!("name_id:{name_id}"),
    }
}

pub fn format_eval_target(target: &F3eEvalTarget<'_>) -> String {
    match target {
        F3eEvalTarget::Cell(cell) => format!("cell:{cell}"),
        F3eEvalTarget::Name { id, .. } => format!("name_id:{id}"),
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

pub fn spill_shape_name(delta: &FecShapeDelta) -> &'static str {
    match &delta.spill_event {
        SpillDeltaEvent::None => "none",
        SpillDeltaEvent::SpillTakeover { .. } => "spill_takeover",
        SpillDeltaEvent::SpillClearance { .. } => "spill_clearance",
        SpillDeltaEvent::SpillBlocked { .. } => "spill_blocked",
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

fn validate_trace_fields(fields: &[(&str, String)]) -> bool {
    if fields.is_empty() {
        return false;
    }
    let mut keys = std::collections::BTreeSet::new();
    for (key, _) in fields {
        if key.trim().is_empty() || !keys.insert(*key) {
            return false;
        }
    }
    true
}
