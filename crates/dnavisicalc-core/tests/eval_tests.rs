use dnavisicalc_core::{Engine, Value};

#[test]
fn evaluates_range_sum_and_if() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("set A1");
    engine.set_number_a1("A2", 20.0).expect("set A2");
    engine
        .set_formula_a1("B1", "@SUM(A1...A2)")
        .expect("set B1 formula");
    engine
        .set_formula_a1("B2", "@IF(B1>25,1,0)")
        .expect("set B2 formula");

    let b1 = engine.cell_state_a1("B1").expect("query B1");
    let b2 = engine.cell_state_a1("B2").expect("query B2");
    assert_eq!(b1.value, Value::Number(30.0));
    assert_eq!(b2.value, Value::Number(1.0));
}

#[test]
fn evaluates_boolean_functions() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "@AND(1, @NOT(0), @OR(0,1))")
        .expect("set formula");
    let a1 = engine.cell_state_a1("A1").expect("query");
    assert_eq!(a1.value, Value::Bool(true));
}

#[test]
fn evaluates_average_and_count() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 2.0).expect("set A1");
    engine.set_number_a1("A2", 4.0).expect("set A2");
    engine
        .set_formula_a1("B1", "AVERAGE(A1:A2)")
        .expect("set B1");
    engine.set_formula_a1("B2", "COUNT(A1:A2)").expect("set B2");

    let b1 = engine.cell_state_a1("B1").expect("query B1");
    let b2 = engine.cell_state_a1("B2").expect("query B2");
    assert_eq!(b1.value, Value::Number(3.0));
    assert_eq!(b2.value, Value::Number(2.0));
}
