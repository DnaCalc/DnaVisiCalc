use dnavisicalc_engine::{
    CellRange, CellRef, CellState, ChartDefinition, ControlDefinition, Engine, RecalcMode, Value,
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
