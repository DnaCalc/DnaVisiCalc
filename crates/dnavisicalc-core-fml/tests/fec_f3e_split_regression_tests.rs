use dnavisicalc_core::{Engine, Value};

fn assert_number(value: &Value, expected: f64) {
    match value {
        Value::Number(actual) => assert!((actual - expected).abs() < 1e-9),
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

#[test]
fn none_profile_formula_path_is_stable() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "SIN(0)").unwrap();

    let state = engine.cell_state_a1("A1").unwrap();
    assert_number(&state.value, 0.0);
}

#[test]
fn ref_only_formula_path_is_stable() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 2.0).unwrap();
    engine.set_formula_a1("B1", "A1*3").unwrap();
    assert_number(&engine.cell_state_a1("B1").unwrap().value, 6.0);

    engine.set_number_a1("A1", 4.0).unwrap();
    assert_number(&engine.cell_state_a1("B1").unwrap().value, 12.0);
}

#[test]
fn name_formula_path_is_stable() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 120.0).unwrap();
    engine.set_name_formula("BASE_TOTAL", "=A1*1.1").unwrap();
    engine.set_name_number("TAX_RATE", 0.2).unwrap();
    engine
        .set_name_formula("GRAND_TOTAL", "=BASE_TOTAL*(1+TAX_RATE)")
        .unwrap();
    engine.set_formula_a1("B1", "=GRAND_TOTAL").unwrap();

    assert_number(&engine.cell_state_a1("B1").unwrap().value, 158.4);
}
