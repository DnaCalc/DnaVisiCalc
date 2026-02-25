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

#[test]
fn let_rejects_invalid_binding_name_conflicts() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=LET(SUM,1,SUM)")
        .expect("set formula");

    let value = engine.cell_state_a1("A1").expect("query").value;
    match value {
        Value::Error(err) => assert!(err.to_string().contains("binding name")),
        other => panic!("expected error value, got {other:?}"),
    }
}

#[test]
fn map_rejects_incompatible_array_shapes_from_lambda_results() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_number_a1("A2", 20.0).expect("A2");
    engine
        .set_formula_a1(
            "B1",
            "=MAP(A1:A2,LAMBDA(x,OFFSET(A1,0,0,IF(x=10,1,2),IF(x=10,2,1))))",
        )
        .expect("set formula");

    let value = engine.cell_state_a1("B1").expect("query").value;
    assert!(matches!(value, Value::Error(_)));
}

#[test]
fn indirect_r1c1_relative_requires_cell_context() {
    let mut engine = Engine::new();
    engine
        .set_name_formula("REL_REF", "=INDIRECT(\"RC\",FALSE)")
        .expect("set name formula");
    engine
        .set_formula_a1("A1", "=REL_REF")
        .expect("set formula");

    let value = engine.cell_state_a1("A1").expect("query").value;
    assert!(matches!(value, Value::Error(_)));
}
