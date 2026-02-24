use dnavisicalc_core::{CellInput, CellRef, Engine, EngineError, RecalcMode, Value};

#[test]
fn clear_resets_cells_and_values() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 5.0).expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");

    engine.clear();

    assert_eq!(engine.all_cell_inputs().len(), 0);
    assert_eq!(
        engine.cell_state_a1("A1").expect("query").value,
        Value::Blank
    );
}

#[test]
fn cell_input_accessors_roundtrip() {
    let mut engine = Engine::new();
    engine
        .set_cell_input_a1("C3", CellInput::Formula("=1+2".to_string()))
        .expect("set C3");

    let input = engine.cell_input_a1("C3").expect("query C3");
    assert_eq!(input, Some(CellInput::Formula("=1+2".to_string())));
    assert_eq!(
        engine
            .formula_source_a1("C3")
            .expect("formula source")
            .expect("formula exists"),
        "=1+2"
    );
}

#[test]
fn out_of_bounds_cellref_errors_on_direct_api() {
    let mut engine = Engine::new();
    let bad = CellRef { col: 0, row: 1 };
    let err = engine
        .set_number(bad, 1.0)
        .expect_err("expected bounds error");
    assert!(matches!(err, EngineError::OutOfBounds(_)));
}

#[test]
fn manual_mode_allows_stale_after_clear_cell() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 1.0).expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");
    engine.recalculate().expect("recalc");

    engine.clear_cell_a1("A1").expect("clear A1");
    let state = engine.cell_state_a1("B1").expect("query B1");
    assert!(state.stale);
}
