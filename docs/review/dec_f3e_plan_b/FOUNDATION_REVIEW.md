# FEC/F3E Redesign Review (DnaVisiCalc 4d4c7a6 vs 9eac9e6)

## Scope and sources
- Repo: `C:\Work\DnaCalc\DnaVisiCalc`
- Baseline: `9eac9e6`
- Redesign: `4d4c7a6`
- Read set:
  - `docs/ENGINE_FEC_F3E_REDESIGN_SPEC.md`
  - `docs/ENGINE_FEC_F3E_REDESIGN_OBSERVATIONS.md`
  - `docs/testing/TESTING_ROUNDS.md` (Round 27)
  - `artifacts/fec_f3e/exams_20260308_redesign/EXAM_SUMMARY.md`
  - `artifacts/fec_f3e/seam_trace.log`
  - `artifacts/fec_f3e/seam_trace.event_counts.tsv`
  - `artifacts/fec_f3e/seam_trace.callgraph.edges.csv`
  - Implementation files in `crates/dnavisicalc-core-fml/src/fec_f3e/`, `src/engine.rs`, `src/eval.rs`, and seam scenario tests.

## 1) Findings (ordered by severity)

### Critical
1. Snapshot fence is session-local, not coordinator-global; this is unsafe for async/MVCC adoption.
   - `fec_host.commit` only compares `tx.snapshot_epoch` against the session snapshot, not against any current coordinator snapshot/version (`fec_host.rs:253-259`, `268-273`).
   - For concurrent mutation + long-running eval, stale transactions can still apply if token remains unchanged.
   - This is a blocker for using the seam as the core concurrency boundary.

2. Name-path runtime dependency deltas are captured but not integrated into incremental invalidation.
   - Deltas include names (`contracts.rs:152-153`, `fec_host.rs:409-417`), but engine application only mutates `runtime_reverse_deps` for cell/spill-child edges (`engine.rs:568-606`).
   - `compute_dirty_closure` propagates only from dirty cells (`engine.rs:2781-2813`), while names are recomputed only when directly dirty (`engine.rs:1877-1891`).
   - This can miss selective invalidation for dynamic name references.

3. Non-formula names currently flow through transactional name eval and hit `RejectedTokenMismatch`.
   - `evaluate_name_via_f3e` uses `expected_token_for(...).unwrap_or(0)` and always opens a token-bound session (`engine.rs:528-541`).
   - If no registered plan exists, commit rejects as token mismatch (`fec_host.rs:283-289`).
   - Trace evidence shows this path: `name:TAX_RATE ... status=RejectedTokenMismatch` (`seam_trace.log` seq 375/377 and seq 202/205).
   - This is a contract ambiguity (literal/static names vs formula names) and a migration risk.

### High
4. Failure semantics are deterministic but under-specified for diagnosis/retry.
   - Status codes exist (`contracts.rs:187-193`), but rejection payload lacks structured conflict detail (expected/current token, expected/current snapshot, missing formula/session reason).
   - `RejectedSnapshotConflict` conflates session-missing, formula mismatch, and epoch mismatch (`fec_host.rs:233-238`, `291-295`).

5. Capability-denied behavior is deterministic but lossy at publication boundary.
   - Execute emits a tagged denied error (`f3e_engine.rs:102-104`), but commit rejection returns generic `#REF!`-style payload (`fec_host.rs:361-368`) and engine publishes commit value on reject (`engine.rs:494-499`).
   - Deterministic yes; diagnostic quality and policy semantics are not stable enough for cross-engine conformance.

### Medium
6. Spill-shape delta is useful but too coarse for selective invalidation without full fallback.
   - Current delta class (`None|Created|Resized|Cleared`) is present (`contracts.rs:169-185`, `fec_host.rs:434-485`), and engine forces full recalc when any spill-shape change occurs during incremental recalc (`engine.rs:2048-2051`).
   - This is correct-by-conservative-fallback today, but insufficient alone for scalable selective invalidation.

7. Rejection branches are implemented but not stress-tested in adversarial packs.
   - Round 27 explicitly notes this gap (`TESTING_ROUNDS.md:480-481`), and redesign observations request adversarial lanes (`ENGINE_FEC_F3E_REDESIGN_OBSERVATIONS.md:14-15`).

## 2) Go / No-Go
- Decision: **Conditional Go**.
- Go for direction/seam shape: `prepare -> open_session -> capability_view -> execute -> commit` is sound and cleaner than pre-redesign (`ENGINE_FEC_F3E_REDESIGN_SPEC.md:6-10`, seam event counts show stable phase cadence).
- No-Go for core seam promotion **until Critical findings 1-3 are resolved**.

## 3) Contract gaps/ambiguities by phase
- `prepare`:
  - Good deterministic token generation, but token semantics mix plan hash and runtime revision indirectly; needs explicit token model for replay/migration.
- `open_session`:
  - Returns opaque session id without validation result type; cannot distinguish non-registered formula upfront.
- `capability_view`:
  - Decision surface is deterministic but not versioned/epoch-bound; missing capability epoch/hash for replay.
- `execute`:
  - Carries decision and observations; good foundation.
  - For denied, runtime is deterministic but should preserve typed denial reason through commit.
- `commit`:
  - Too few rejection classes; conflates causes.
  - Missing structured conflict payload and global snapshot fence.

## 4) Dependency/spill delta sufficiency
- Runtime dependency delta sufficiency: **partially sufficient**.
  - Sufficient for dynamic cell retargeting and spill-child edge shifts in current cell scheduler.
  - Not sufficient for name-driven selective invalidation (captured, not consumed).
- Spill-shape delta sufficiency: **insufficient for selective policy alone**.
  - Adequate as a conservative trigger (full fallback), not enough for fine-grained invalidation/scheduling.

## 5) Failure-mode review
- Token mismatch:
  - Deterministic and exercised (trace shows explicit rejected cases).
  - Needs sub-status (expected-mismatch vs unregistered-formula) for migration/debug safety.
- Snapshot conflict:
  - Deterministic branch exists but adversarially under-tested; also not globally fenced to coordinator epoch.
- Capability denial:
  - Deterministic branch exists (`RejectedCapabilityDenied`) but denial cause details are lost at commit publication.

## 6) Performance concerns and required counters/benchmarks
- Concerns:
  - Full recalc fallback on any spill-shape change can dominate mixed dynamic-array workloads.
  - Session open/commit per eval may become overhead under high fan-out/parallel eval.
  - Rejection/rollback behavior has no measured contention profile.
- Required counters before wider adoption:
  1. `fec.commit.status_count{Applied,RejectedTokenMismatch,RejectedSnapshotConflict,RejectedCapabilityDenied,...}`
  2. `fec.commit.reject_reason_count{session_missing,formula_missing,session_formula_mismatch,token_expected_mismatch,token_tx_mismatch,snapshot_mismatch,capability_denied}`
  3. `fec.commit.dep_delta_spill_children` (already suggested in observations).
  4. `engine.incremental.full_fallback_reason_count{spill_shape_change,...}`
  5. `engine.runtime_reverse_deps.edge_count` and churn rates.
- Required benchmarks:
  1. Dynamic retargeting churn (`INDIRECT`/`OFFSET`) with incremental-only path.
  2. Spill oscillation workloads (expand/shrink) with visible fallback ratios.
  3. Name-heavy dependency workloads (including dynamic name reads).
  4. Synthetic contention harness with delayed execute/commit against advancing epochs.

## 7) Required contract edits before acceptance (field/type-level)
1. Add explicit snapshot fence to commit:
   - `EvalTransaction`: add `structure_epoch` or `coordinator_snapshot_epoch`.
   - `commit`: compare against current coordinator epoch/version, not only session snapshot.
2. Split rejection status taxonomy:
   - Add `RejectedSessionNotFound`, `RejectedFormulaNotRegistered`, `RejectedFormulaMismatch`, `RejectedSnapshotStale`.
3. Add structured reject metadata:
   - `CommitResult.reject_detail` with fields for expected/current token and expected/current snapshot.
4. Clarify non-formula target contract:
   - Either disallow transactional name eval for non-formula names, or support `expected_token: None` path with non-rejecting commit semantics.
5. Preserve capability denial detail through commit:
   - Include denied capability tag and policy class in commit result.
6. Extend dependency publication:
   - Add optional name-delta publication channel wired to scheduler invalidation APIs.
7. Extend spill metadata for selective invalidation:
   - Include explicit affected region diff (`old_range`, `new_range`, possibly derived entered/exited cells or region token).

## 8) Minimal cross-repo adoption plan (phased)
1. Phase A: Compatibility seam shim (no behavior changes)
   - Introduce F3E/F3C terminology aliasing (`F3E` canonical, `F3C` compatibility typedefs/wrappers where needed).
   - Keep current call shape; add new optional fields/status values behind defaults.
2. Phase B: Deterministic failure hardening
   - Implement split reject statuses + reject detail payload.
   - Add adversarial transaction pack (token mismatch, snapshot stale, capability denial).
3. Phase C: Name/runtime invalidation parity
   - Wire name dependency deltas into incremental invalidation.
   - Add name-dynamic retargeting tests.
4. Phase D: Spill selective policy prep
   - Emit richer spill delta metadata; keep conservative fallback as default.
   - Add policy flag for selective spill invalidation experiments.
5. Phase E: Async/MVCC pilot
   - Enforce global snapshot fences in commit.
   - Run contention benchmark suite and deterministic replay packs.
6. Phase F: Foundation synthesis promotion
   - Promote stabilized contract text into Foundation core model docs and pack contracts only after Phase B+C+D acceptance.

## 9) Targeted answer on conditional formatting and `TEXT`
- Current design lane should treat this as: `TEXT(value, format_text)` depends on explicit `format_text`; conditional formatting is not formula-visible by default.
- If a profile later exposes effective style to formulas, model it as a profile-gated formatting overlay token family (not ambient implicit behavior).