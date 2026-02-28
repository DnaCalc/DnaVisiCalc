use dnavisicalc_engine::{
    CellRange, CellRef, CellState, ChartDefinition, ControlDefinition,
    DIAG_CIRCULAR_REFERENCE_DETECTED, Engine, EngineChangeEvent, IterationConfig, REJECT_KIND_NONE,
    REJECT_KIND_STRUCTURAL_CONSTRAINT, RecalcMode, STATUS_REJECT_POLICY,
    STATUS_REJECT_STRUCTURAL_CONSTRAINT, STRUCT_OP_INSERT_ROW, Value,
};

fn assert_epoch_ordering(engine: &Engine) {
    assert!(
        engine.stabilized_epoch() <= engine.committed_epoch(),
        "INV-EPOCH-001 violated: stabilized_epoch must be <= committed_epoch"
    );
}

#[test]
fn ct_epoch_001_epoch_ordering_holds() {
    let mut engine = Engine::new();
    assert_epoch_ordering(&engine);

    engine.set_number_a1("A1", 10.0).expect("set A1");
    assert_epoch_ordering(&engine);

    engine.set_formula_a1("B1", "=A1*2").expect("set B1");
    assert_epoch_ordering(&engine);
}

#[test]
fn ct_epoch_002_epoch_monotonicity_holds() {
    let mut engine = Engine::new();
    let mut prev_committed = engine.committed_epoch();
    let mut prev_stabilized = engine.stabilized_epoch();

    let steps = [
        ("A1", 1.0),
        ("A2", 2.0),
        ("A3", 3.0),
        ("B1", 10.0),
        ("B2", 20.0),
    ];
    for (addr, value) in steps {
        engine.set_number_a1(addr, value).expect("set number");
        let committed = engine.committed_epoch();
        let stabilized = engine.stabilized_epoch();
        assert!(
            committed >= prev_committed,
            "INV-EPOCH-002 committed regression"
        );
        assert!(
            stabilized >= prev_stabilized,
            "INV-EPOCH-002 stabilized regression"
        );
        prev_committed = committed;
        prev_stabilized = stabilized;
    }
}

#[test]
fn ct_cell_001_stale_flag_definition_holds() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 7.0).expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");

    let state = engine.cell_state_a1("B1").expect("query B1");
    let expected_stale = state.value_epoch < engine.committed_epoch();
    assert_eq!(
        state.stale, expected_stale,
        "INV-CELL-001 violated: stale must match (value_epoch < committed_epoch)"
    );
}

fn run_det_script() -> (
    Vec<CellState>,
    Vec<(dnavisicalc_engine::CellRef, dnavisicalc_engine::CellInput)>,
) {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 2.0).expect("set A1");
    engine.set_number_a1("A2", 3.0).expect("set A2");
    engine.set_formula_a1("B1", "=A1+A2").expect("set B1");
    engine.set_formula_a1("B2", "=B1*10").expect("set B2");
    let _ = engine.recalculate();

    let states = ["A1", "A2", "B1", "B2"]
        .into_iter()
        .map(|a1| engine.cell_state_a1(a1).expect("cell state"))
        .collect::<Vec<_>>();
    let inputs = engine.all_cell_inputs();
    (states, inputs)
}

fn state_number(state: &CellState) -> f64 {
    match state.value {
        Value::Number(value) => value,
        ref other => panic!("expected numeric value, got {other:?}"),
    }
}

#[test]
fn ct_det_001_replay_determinism_smoke() {
    let (left_states, left_inputs) = run_det_script();
    let (right_states, right_inputs) = run_det_script();

    assert_eq!(
        left_inputs, right_inputs,
        "INV-DET-001 input replay mismatch"
    );
    assert_eq!(
        left_states.len(),
        right_states.len(),
        "INV-DET-001 state length mismatch"
    );
    for (l, r) in left_states.into_iter().zip(right_states.into_iter()) {
        assert_eq!(l.stale, r.stale, "INV-DET-001 stale mismatch");
        assert_eq!(
            l.value_epoch, r.value_epoch,
            "INV-DET-001 value_epoch mismatch"
        );
        assert_eq!(l.value, r.value, "INV-DET-001 value mismatch");
    }
}

#[test]
fn ct_temporal_001_manual_recalc_eventually_stabilizes() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_formula_a1("A1", "=1+1").expect("set A1");

    let before = engine.cell_state_a1("A1").expect("before");
    let _ = engine.recalculate();
    let after = engine.cell_state_a1("A1").expect("after");

    assert!(matches!(after.value, Value::Number(_)));
    assert!(
        !after.stale || after.value_epoch == engine.committed_epoch(),
        "TEMP-RECALC-001 violated: recalc should eventually stabilize current epoch"
    );
    assert!(
        after.value_epoch >= before.value_epoch,
        "TEMP-RECALC-001 violated: value_epoch regressed"
    );
}

#[test]
fn ct_str_001_rejected_structural_op_is_atomic_noop() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine
        .set_formula_a1("A1", "=SEQUENCE(3,1)")
        .expect("set spill formula");
    engine.recalculate().expect("initial recalc");

    let before_epoch = engine.committed_epoch();
    let before_a1 = engine.cell_input_a1("A1").expect("A1 before");
    let before_spill = engine
        .spill_range_for_cell(CellRef::from_a1("A1").expect("A1"))
        .expect("spill range before");

    let result = engine.insert_row(2);
    let status = match result {
        Ok(()) => panic!("insert_row(2) should be rejected for active spill boundary"),
        Err(dnavisicalc_engine::EngineError::Api { status, .. }) => status,
        Err(other) => panic!("unexpected insert_row error: {other}"),
    };
    let reject_ctx = engine.last_reject_context();

    assert!(
        status == STATUS_REJECT_STRUCTURAL_CONSTRAINT || status == STATUS_REJECT_POLICY,
        "expected reject status, got {status}"
    );
    assert_eq!(
        engine.committed_epoch(),
        before_epoch,
        "INV-STR-001 violated: rejected structural op changed committed_epoch"
    );
    assert_eq!(
        engine.cell_input_a1("A1").expect("A1 after"),
        before_a1,
        "INV-STR-001 violated: rejected structural op changed inputs"
    );
    assert_eq!(
        engine
            .spill_range_for_cell(CellRef::from_a1("A1").expect("A1"))
            .expect("spill range after"),
        before_spill,
        "INV-STR-001 violated: rejected structural op changed spill range"
    );

    assert_eq!(
        reject_ctx.reject_kind, REJECT_KIND_STRUCTURAL_CONSTRAINT,
        "INV-STR-001 violated: reject context kind mismatch"
    );
    assert_eq!(
        reject_ctx.op_kind, STRUCT_OP_INSERT_ROW,
        "INV-STR-001 violated: reject op kind mismatch"
    );
    assert_eq!(
        reject_ctx.op_index, 2,
        "INV-STR-001 violated: reject op index mismatch"
    );
}

#[test]
fn ct_cycle_001_non_iterative_cycle_is_nonfatal_and_diagnostic() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_iteration_config(IterationConfig {
        enabled: false,
        max_iterations: 64,
        convergence_tolerance: 0.000_001,
    });
    engine
        .change_tracking_enable()
        .expect("enable change tracking");

    engine.set_formula_a1("A1", "=B1+1").expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");
    engine.recalculate().expect("non-iterative recalc");

    let a1 = engine.cell_state_a1("A1").expect("A1");
    let b1 = engine.cell_state_a1("B1").expect("B1");
    assert!(
        matches!(a1.value, Value::Number(_)),
        "INV-CYCLE-001 violated: A1 should be non-fatal numeric fallback"
    );
    assert!(
        matches!(b1.value, Value::Number(_)),
        "INV-CYCLE-001 violated: B1 should be non-fatal numeric fallback"
    );

    let changes = engine.drain_change_events().expect("drain changes");
    let diag_count = changes
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                EngineChangeEvent::Diagnostic { code, .. }
                    if *code == DIAG_CIRCULAR_REFERENCE_DETECTED
            )
        })
        .count();
    assert!(
        diag_count > 0,
        "INV-CYCLE-001 violated: expected at least one cycle diagnostic entry"
    );
}

#[test]
fn ct_temp_stream_001_without_ticks_stream_does_not_advance() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine
        .set_formula_a1("A1", "=STREAM(1)")
        .expect("set stream");
    engine.recalculate().expect("initial recalc");

    let initial = state_number(&engine.cell_state_a1("A1").expect("A1 initial"));
    for _ in 0..4 {
        engine.recalculate().expect("recalc without tick");
        let current = state_number(&engine.cell_state_a1("A1").expect("A1 current"));
        assert_eq!(
            current, initial,
            "TEMP-STREAM-001 violated: stream advanced without tick_streams"
        );
    }
}

#[test]
fn ct_temp_stream_002_tick_advances_stream_after_recalc() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine
        .set_formula_a1("A1", "=STREAM(1)")
        .expect("set stream");
    engine.recalculate().expect("initial recalc");

    let before_tick = state_number(&engine.cell_state_a1("A1").expect("A1 before tick"));
    assert!(
        engine.tick_streams(1.2),
        "tick_streams should advance stream"
    );
    let before_recalc = state_number(&engine.cell_state_a1("A1").expect("A1 before recalc"));
    assert_eq!(
        before_recalc, before_tick,
        "TEMP-STREAM-002 violated: manual mode should not apply tick until recalc"
    );

    engine.recalculate().expect("recalc after tick");
    let after_recalc = state_number(&engine.cell_state_a1("A1").expect("A1 after recalc"));
    assert!(
        after_recalc > before_tick,
        "TEMP-STREAM-002 violated: stream value did not advance after tick + recalc"
    );
}

#[test]
fn ct_temp_reject_001_rejected_structural_call_has_no_observable_mutation() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine
        .change_tracking_enable()
        .expect("enable change tracking");
    engine
        .set_formula_a1("A1", "=SEQUENCE(3,1)")
        .expect("set spill formula");
    engine.recalculate().expect("initial recalc");
    let _ = engine
        .drain_change_events()
        .expect("drain baseline changes");

    let before_epoch = engine.committed_epoch();
    let before_b1 = engine.cell_state_a1("B1").expect("B1 before");
    let reject = engine.insert_row(2);
    assert!(reject.is_err(), "expected rejected structural op");

    let after_epoch = engine.committed_epoch();
    let after_b1 = engine.cell_state_a1("B1").expect("B1 after");
    let changes = engine.drain_change_events().expect("drain changes");

    assert_eq!(
        before_epoch, after_epoch,
        "TEMP-REJECT-001 violated: rejected call changed epoch"
    );
    assert_eq!(
        before_b1, after_b1,
        "TEMP-REJECT-001 violated: rejected call changed unrelated cell state"
    );
    assert!(
        changes.is_empty(),
        "TEMP-REJECT-001 violated: rejected call emitted change entries"
    );
}

#[test]
fn ct_temp_vol_001_volatile_formula_does_not_self_tick() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_formula_a1("A1", "=RAND()").expect("set RAND");
    engine.recalculate().expect("initial recalc");

    let baseline = state_number(&engine.cell_state_a1("A1").expect("A1 baseline"));
    for _ in 0..8 {
        let state = engine.cell_state_a1("A1").expect("A1 reread");
        let current = state_number(&state);
        assert_eq!(
            current, baseline,
            "TEMP-VOL-001 violated: volatile cell changed without explicit trigger"
        );
        assert_eq!(
            engine.last_reject_kind(),
            REJECT_KIND_NONE,
            "unexpected reject kind while idle"
        );
        assert_eq!(
            engine.last_error_kind(),
            0,
            "unexpected non-ok error kind while idle"
        );
    }
    engine.recalculate().expect("explicit recalc trigger");
}

#[test]
fn ct_entities_001_control_roundtrip_holds() {
    let mut engine = Engine::new();

    engine
        .define_control("speed", ControlDefinition::slider(0.0, 10.0, 0.5))
        .expect("define speed control");
    engine
        .define_control("apply", ControlDefinition::button())
        .expect("define apply control");

    assert_eq!(engine.control_value("speed"), Some(0.0));
    assert_eq!(
        engine.control_definition("speed"),
        Some(ControlDefinition::slider(0.0, 10.0, 0.5))
    );

    engine
        .set_control_value("speed", 9.0)
        .expect("set speed control value");
    assert_eq!(engine.control_value("speed"), Some(9.0));

    let controls = engine.all_controls();
    assert_eq!(controls.len(), 2, "control iterator length mismatch");
    assert_eq!(controls[0].0, "APPLY", "controls must be sorted by name");
    assert_eq!(controls[1].0, "SPEED", "controls must be sorted by name");

    assert!(engine.remove_control("speed"), "control should be removed");
    assert!(
        !engine.remove_control("speed"),
        "control removal should report not found on second call"
    );
    assert_eq!(engine.control_value("speed"), None);
    assert_eq!(engine.control_definition("speed"), None);
}

#[test]
fn ct_entities_002_chart_roundtrip_holds() {
    let mut engine = Engine::new();
    let start = CellRef::from_a1("A1").expect("A1");
    let end = CellRef::from_a1("B3").expect("B3");
    let range = CellRange::new(start, end);

    engine
        .define_chart(
            "sales",
            ChartDefinition {
                source_range: range,
            },
        )
        .expect("define chart");

    let charts = engine.all_charts();
    assert_eq!(charts.len(), 1, "chart iterator length mismatch");
    assert_eq!(charts[0].0, "SALES");
    assert_eq!(charts[0].1.source_range, range);

    assert!(engine.remove_chart("sales"), "chart should be removed");
    assert!(
        !engine.remove_chart("sales"),
        "chart removal should report not found on second call"
    );
    assert!(engine.all_charts().is_empty(), "charts should be empty");
}
