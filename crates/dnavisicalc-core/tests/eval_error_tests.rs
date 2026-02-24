use dnavisicalc_core::{Engine, Value};

#[test]
fn division_by_zero_returns_error_value() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=1/0").expect("set formula");
    let state = engine.cell_state_a1("A1").expect("query A1");
    match state.value {
        Value::Error(err) => assert!(err.to_string().contains("division by zero")),
        other => panic!("expected error value, got {other:?}"),
    }
}

#[test]
fn unknown_function_returns_name_error() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=MYSTERY(1)")
        .expect("set formula");
    let state = engine.cell_state_a1("A1").expect("query A1");
    match state.value {
        Value::Error(err) => assert!(err.to_string().contains("unknown function")),
        other => panic!("expected error value, got {other:?}"),
    }
}

#[test]
fn function_arity_errors_are_reported() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=IF(1,2)")
        .expect("set formula");
    engine
        .set_formula_a1("A2", "=NOT(1,2)")
        .expect("set formula");

    let a1 = engine.cell_state_a1("A1").expect("query A1");
    let a2 = engine.cell_state_a1("A2").expect("query A2");
    assert!(matches!(a1.value, Value::Error(_)));
    assert!(matches!(a2.value, Value::Error(_)));
}

#[test]
fn scalar_range_expression_returns_value_error() {
    let mut engine = Engine::new();
    engine.set_formula_a1("B1", "=A1:A2").expect("set formula");

    let value = engine.cell_state_a1("B1").expect("query").value;
    assert!(matches!(value, Value::Error(_)));
}
