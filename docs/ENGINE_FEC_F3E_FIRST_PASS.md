# Engine Internal FEC/F3E Split (First Pass)

## Purpose
This note documents the first-pass internal split introduced for exploratory work in `rust-fml` by routing `dnavisicalc-core-fml` parse/dependency/evaluation flow through explicit FEC/F3E interfaces while preserving the public API behavior.

Scope is intentionally incremental and aligned to v0 stability requirements.

Follow-up planning for deeper API seam examination and instrumentation:
- `docs/ENGINE_FEC_F3E_API_EXAMINATION_PLAN.md`

Implementation location for this exploratory split:
- semantic crate: `crates/dnavisicalc-core-fml`
- C ABI wrapper crate: `engines/rust/coreengine-rust-fml`

## Current Mapping

### F3E (Formula-Function-Formatting semantic layer)
- Primary modules:
  - `crates/dnavisicalc-core-fml/src/parser.rs`
  - `crates/dnavisicalc-core-fml/src/ast.rs`
  - `crates/dnavisicalc-core-fml/src/eval.rs`
  - `crates/dnavisicalc-core-fml/src/deps.rs` (dependency extraction logic)
  - `crates/dnavisicalc-core-fml/src/fec_f3e/f3e_engine.rs` (`CoreF3eEngine` adapter)
  - `crates/dnavisicalc-core-fml/src/fec_f3e/contracts.rs` (shared interface contracts)
  - `crates/dnavisicalc-core-fml/src/fec_f3e/spec.rs` (spec clause markers/version)
- Ownership:
  - parse/bind and expression forms,
  - formula dependency intent extraction,
  - value/type semantics and runtime evaluation behavior.

### FEC (host/context/dependency-routing layer)
- Primary modules:
  - `crates/dnavisicalc-core-fml/src/engine.rs` (orchestration host)
  - `crates/dnavisicalc-core-fml/src/fec_f3e/fec_host.rs` (`DefaultFecHost` adapter)
  - `crates/dnavisicalc-core-fml/src/fec_f3e/contracts.rs` (host-side interface contracts)
- Ownership:
  - capability-view construction,
  - dependency registration token routing,
  - publication of F3E evaluation output into engine-visible values and epochs.

## New Internal Interface Scaffolding
- Added internal module:
  - `crates/dnavisicalc-core-fml/src/fec_f3e/`
- File-level split:
  - `contracts.rs` (traits and cross-boundary data contracts)
  - `f3e_engine.rs` (F3E implementation adapter)
  - `fec_host.rs` (FEC host adapter)
  - `spec.rs` (spec IDs/version markers for interface formalization)
  - `mod.rs` (module re-export boundary)
- Introduced traits/types:
  - `F3eEngine` with:
    - `compile(...)`
    - `declare_dependencies(...)`
    - `evaluate(...)`
  - `FecHost` with:
    - `capability_view(...)`
    - `register_dependencies(...)`
    - `publish_result(...)`
  - Shared context/result and capability types:
    - `F3eCompileContext`
    - `F3eCompiledFormula`
    - `F3eDeclaredDependencies`
    - `F3eEvalContext`
    - `F3eEvalResult`
    - `FecCapabilityTag`
    - `ScopedCapabilityView`
    - `FecFormulaId`

## First-Pass Runtime Routing Changes
- Formula set paths now go through F3E compile/declare:
  - `Engine::set_formula`
  - `Engine::set_name_formula`
- Recalc cell/name evaluation now goes through F3E evaluate + FEC publish:
  - full recalc and incremental recalc loops in `engine.rs`.
- FEC dependency registrations are refreshed on structural rewrite rebuilds.

## Capability View Baseline
Current `DefaultFecHost` provides a minimal capability view aligned with current engine behavior:
- provided: `ReferenceResolution`, `CallerContext`, `TimeProvider`, `RandomProvider`, `ExternalProvider`, `LocaleParseFormat`
- deferred/stubbed:
  - `FeatureGate` (TODO)
  - `ErrorDetailEnrichment` (TODO)

This first pass does not enforce capability denial yet; behavior remains compatibility-preserving.

## Deferred Items (Tracked by TODO Markers)
- Capability-denial policy mapping (deterministic value-level contract).
- Finer caller-context modeling beyond explicit `ROW`/`COLUMN` detection.
- Time/random/external capability partition refinement (especially UDF-linked lanes).
- Feature-gate and enriched-diagnostics capability lanes.
- Optional dynamic-dependency token refinement beyond static extraction.

## Non-Goals of This Pass
- No external C API contract changes (`docs/ENGINE_API.md` unchanged).
- No broad evaluator rewrite.
- No value semantics moved into host layer.

## Test Coverage Added
- New regression tests:
  - `crates/dnavisicalc-core-fml/tests/fec_f3e_split_regression_tests.rs`
- New internal engine tests:
  - FEC registration profile classification (`none`, `ref_only`) and structural-refresh behavior in `engine.rs`.
