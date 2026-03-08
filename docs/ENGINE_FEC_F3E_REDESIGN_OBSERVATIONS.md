# FEC/F3E Redesign Observations and Suggestions

## Current Observations (Round 27)
1. Runtime dependency deltas now materially improve dynamic-reference invalidation.
   - `INDIRECT`/`OFFSET` target flips are reflected in observed `dep_delta_cells`.
2. Spill transitions are now explicit seam metadata.
   - `spill_shape_delta=created` and `spill_shape_delta=cleared` are visible in commit traces.
3. Transaction status handling is deterministic in exercised flows.
   - observed commits were `Applied`; rejection branches are implemented but not yet stress-tested.
4. Call-graph shape is cleaner and phase-oriented.
   - `open_session -> capability_view -> execute -> commit` is consistent across scenarios.

## Suggestions
1. Add a small adversarial test lane for `RejectedTokenMismatch` and `RejectedSnapshotConflict` paths.
2. Add a host policy knob to choose token rotation policy:
   - rotate on any delta (current behavior), or
   - rotate only on structural dependency delta.
3. Add typed trace fields for dependency delta sizes by category:
   - `dep_delta_cells`
   - `dep_delta_names`
   - `dep_delta_spill_children`
4. Extend name-path incremental policy to consume observed name dependency deltas (currently captured, not scheduled).
5. Add replay fixtures for transaction envelopes to enable deterministic seam-level minimization outside full engine tests.

## Evidence References
- `artifacts/fec_f3e/exams_20260308_redesign/EXAM_SUMMARY.md`
- `artifacts/fec_f3e/seam_trace.event_counts.tsv`
- `docs/ENGINE_FEC_F3E_REDESIGN_SPEC.md`
