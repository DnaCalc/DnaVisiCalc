• Go/No-Go: No-Go for freezing as core seam today.
  Direction is strong, but I’d rate it compatible_with_adaptations after the blockers below.

  Findings (ordered by severity)

  1. Critical correctness blocker: normal flows currently record transactional failures for non-
     formula names.
     recalculate_full evaluates all names through the transactional seam, including literal names,
     which have no installed plan, so commit rejects with RejectedTokenMismatch. This is visible in
     the main seam trace (name:TAX_RATE) and should not be counted as a failure path in normal
     operation.
     Refs: engine.rs:1559, engine.rs:520, fec_host.rs:283, seam_trace.log:382, seam_trace.log:387
  2. Critical concurrency blocker: seam is not thread-safe / multi-session-safe enough for core-engine
     concurrency claims.
     Contracts and state are single-thread oriented (Rc, mutable host maps, wrapping session counter),
     and snapshot checks are session-local only. This is fine for current single-thread execution, but
     not sufficient for multi-threaded core adoption.
     Refs: contracts.rs:1, contracts.rs:67, fec_host.rs:201, fec_host.rs:253
  3. High failure-semantics gap: rejection branches are implemented but not properly validated by
     adversarial tests.
     Round 27 itself flags this, and the seam test lane does not directly assert token/snapshot/
     capability rejection behavior.
     Refs: TESTING_ROUNDS.md:480, ENGINE_FEC_F3E_REDESIGN_OBSERVATIONS.md:14,
     fec_f3e_seam_scenarios_tests.rs:1
  4. High incremental-policy gap: runtime dependency deltas are only partially consumed.
     Cell/spill child deltas are applied to incremental closure, but name deltas are not used for
     selective invalidation yet.
     Refs: engine.rs:568, engine.rs:573, fec_host.rs:314, ENGINE_FEC_F3E_REDESIGN_OBSERVATIONS.md:22
  5. Medium contract ambiguity: statuses are overloaded.
     RejectedSnapshotConflict covers multiple distinct causes; RejectedTokenMismatch also covers “plan
     not registered”. This hurts deterministic failure classification and migration diagnostics.
     Refs: fec_host.rs:234, fec_host.rs:291, fec_host.rs:285
  6. Medium performance/evidence gap before wider adoption.
     Spill-shape change forces full recalc fallback (safe, but potentially costly), and current
     callgraph derivation is adjacency-based, not causal.
     Refs: engine.rs:2048, seam_trace.event_counts.tsv:7, derive_trace_graph.ps1:47

  Required contract edits before acceptance

  1. Add explicit formula-kind handling so literal names/cells never open sessions or commit as
     formula plans.
  2. Split commit rejection statuses into distinct causes (SessionNotFound, FormulaNotRegistered,
     SessionFormulaMismatch, TokenMismatch, SnapshotConflict, CapabilityDenied).
  3. Move capability authority fully to FEC session state; commit should verify session-held decision,
     not trust transaction-carried decision alone.
  4. Add coordinator epoch/version check at commit (not only tx-vs-session snapshot equality).
  5. Extend dependency delta contract usage to include name invalidation routing, or explicitly scope
     it out in spec with policy gating.
  6. Replace Rc seam-carrying payloads with thread-ready equivalents (Arc) if this seam is to be core
     in concurrent scheduler contexts.
  7. Add deterministic machine code fields in failures (stable reason code), avoid relying on free-
     text error messages for policy.

  Minimal cross-repo adoption plan (phased)

  1. Hardening in DnaVisiCalc first: fix non-formula name commit path, add adversarial tests for all
     reject statuses, add capability-denial test knob.
  2. Compatibility shim phase: provide shim mapping old compile/declare/evaluate callers to prepare/
     open_session/capability_view/execute/commit so migration from static-declaration model is
     incremental.
  3. Scheduler integration phase: consume name deltas for selective invalidation and validate spill-
     delta fallback policy under load.
  4. Performance gate phase: add benchmarks/counters for per-phase latency, commit status counts,
     delta sizes, token rotations, and spill-triggered full-recalc rate.
  5. Promotion: adopt as core seam only after (a) no intentional failures in nominal traces, (b)
     rejection paths explicitly tested, (c) concurrency assumptions codified.

  Independent verification run: cargo test -p dnavisicalc-core-fml and seam test lanes both passed on
  current 4d4c7a6.