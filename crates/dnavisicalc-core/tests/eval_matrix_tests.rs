use dnavisicalc_core::{Engine, Value};

#[test]
fn comparison_operators_work_for_numbers_and_bools() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=2=2").expect("set formula");
    engine.set_formula_a1("A2", "=2<>3").expect("set formula");
    engine.set_formula_a1("A3", "=2<3").expect("set formula");
    engine.set_formula_a1("A4", "=2<=2").expect("set formula");
    engine.set_formula_a1("A5", "=3>2").expect("set formula");
    engine.set_formula_a1("A6", "=3>=3").expect("set formula");
    engine
        .set_formula_a1("A7", "=TRUE<FALSE")
        .expect("set formula");

    for row in 1..=6 {
        let addr = format!("A{row}");
        let value = engine.cell_state_a1(&addr).expect("query").value;
        assert_eq!(value, Value::Bool(true));
    }
    let bool_cmp = engine.cell_state_a1("A7").expect("query A7").value;
    assert_eq!(bool_cmp, Value::Bool(false));
}

#[test]
fn aggregate_functions_handle_empty_argument_lists() {
    let mut engine = Engine::new();
    engine.set_formula_a1("B1", "=SUM()").expect("set SUM");
    engine.set_formula_a1("B2", "=MIN()").expect("set MIN");
    engine.set_formula_a1("B3", "=MAX()").expect("set MAX");
    engine
        .set_formula_a1("B4", "=AVERAGE()")
        .expect("set AVERAGE");
    engine.set_formula_a1("B5", "=COUNT()").expect("set COUNT");

    for row in 1..=5 {
        let addr = format!("B{row}");
        let value = engine.cell_state_a1(&addr).expect("query").value;
        assert_eq!(value, Value::Number(0.0));
    }
}

#[test]
fn logical_functions_validate_arity_and_truthiness() {
    let mut engine = Engine::new();
    engine.set_formula_a1("C1", "=AND()").expect("set AND");
    engine.set_formula_a1("C2", "=OR()").expect("set OR");
    engine.set_formula_a1("C3", "=NOT(1)").expect("set NOT");
    engine
        .set_formula_a1("C4", "=IF(0,11,22)")
        .expect("set IF false branch");

    assert!(matches!(
        engine.cell_state_a1("C1").expect("query").value,
        Value::Error(_)
    ));
    assert!(matches!(
        engine.cell_state_a1("C2").expect("query").value,
        Value::Error(_)
    ));
    assert_eq!(
        engine.cell_state_a1("C3").expect("query").value,
        Value::Bool(false)
    );
    assert_eq!(
        engine.cell_state_a1("C4").expect("query").value,
        Value::Number(22.0)
    );
}
