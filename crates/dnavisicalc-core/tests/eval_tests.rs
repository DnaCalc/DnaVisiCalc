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

#[test]
fn evaluates_text_concat_operator_and_function() {
    let mut engine = Engine::new();
    engine.set_text_a1("A1", "dna").expect("set A1 text");
    engine
        .set_formula_a1("B1", "=A1&\" calc\"")
        .expect("set B1 formula");
    engine
        .set_formula_a1("B2", "=CONCAT(\"v\", \"1\", \"-\", A1)")
        .expect("set B2 formula");

    let b1 = engine.cell_state_a1("B1").expect("query B1");
    let b2 = engine.cell_state_a1("B2").expect("query B2");
    assert_eq!(b1.value, Value::Text("dna calc".to_string()));
    assert_eq!(b2.value, Value::Text("v1-dna".to_string()));
}

#[test]
fn evaluates_len_function_on_text() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=LEN(\"hello\")")
        .expect("set A1 formula");
    let a1 = engine.cell_state_a1("A1").expect("query A1");
    assert_eq!(a1.value, Value::Number(5.0));
}

#[test]
fn evaluates_math_functions() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=ABS(-3)").expect("ABS");
    engine.set_formula_a1("A2", "=INT(3.9)").expect("INT");
    engine
        .set_formula_a1("A3", "=ROUND(12.345,2)")
        .expect("ROUND");
    engine.set_formula_a1("A4", "=SIGN(-10)").expect("SIGN");
    engine.set_formula_a1("A5", "=SQRT(81)").expect("SQRT");
    engine.set_formula_a1("A6", "=LOG10(1000)").expect("LOG10");
    engine.set_formula_a1("A7", "=LN(EXP(2))").expect("LN/EXP");
    engine.set_formula_a1("A8", "=ATN(1)").expect("ATN");
    engine.set_formula_a1("A9", "=PI()").expect("PI");

    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(3.0)
    );
    assert_eq!(
        engine.cell_state_a1("A2").expect("A2").value,
        Value::Number(3.0)
    );
    assert_eq!(
        engine.cell_state_a1("A3").expect("A3").value,
        Value::Number(12.35)
    );
    assert_eq!(
        engine.cell_state_a1("A4").expect("A4").value,
        Value::Number(-1.0)
    );
    assert_eq!(
        engine.cell_state_a1("A5").expect("A5").value,
        Value::Number(9.0)
    );
    assert_eq!(
        engine.cell_state_a1("A6").expect("A6").value,
        Value::Number(3.0)
    );
    match engine.cell_state_a1("A7").expect("A7").value {
        Value::Number(v) => assert!((v - 2.0).abs() < 1e-9),
        other => panic!("expected numeric A7, got {other:?}"),
    }
    match engine.cell_state_a1("A8").expect("A8").value {
        Value::Number(v) => assert!((v - std::f64::consts::FRAC_PI_4).abs() < 1e-9),
        other => panic!("expected numeric A8, got {other:?}"),
    }
    match engine.cell_state_a1("A9").expect("A9").value {
        Value::Number(v) => assert!((v - std::f64::consts::PI).abs() < 1e-12),
        other => panic!("expected numeric A9, got {other:?}"),
    }
}

#[test]
fn evaluates_financial_functions() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("B1", "=PMT(0.01,12,1000)")
        .expect("PMT");
    engine
        .set_formula_a1("B2", "=FV(0.01,12,-100,0,0)")
        .expect("FV");
    engine
        .set_formula_a1("B3", "=PV(0.01,12,-100,0,0)")
        .expect("PV");
    engine
        .set_formula_a1("B4", "=NPV(0.1,100,110)")
        .expect("NPV");

    match engine.cell_state_a1("B1").expect("B1").value {
        Value::Number(v) => assert!((v + 88.84878867834166).abs() < 1e-9),
        other => panic!("expected numeric B1, got {other:?}"),
    }
    match engine.cell_state_a1("B2").expect("B2").value {
        Value::Number(v) => assert!((v - 1268.2503013196976).abs() < 1e-9),
        other => panic!("expected numeric B2, got {other:?}"),
    }
    match engine.cell_state_a1("B3").expect("B3").value {
        Value::Number(v) => assert!((v - 1125.5077473484644).abs() < 1e-9),
        other => panic!("expected numeric B3, got {other:?}"),
    }
    match engine.cell_state_a1("B4").expect("B4").value {
        Value::Number(v) => assert!((v - 181.8181818181818).abs() < 1e-9),
        other => panic!("expected numeric B4, got {other:?}"),
    }
}

#[test]
fn evaluates_lookup_and_error_functions() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_number_a1("A2", 1.0).expect("A2");
    engine.set_number_a1("B1", 20.0).expect("B1");
    engine.set_number_a1("B2", 2.0).expect("B2");
    engine.set_number_a1("C1", 30.0).expect("C1");
    engine.set_number_a1("C2", 3.0).expect("C2");
    engine
        .set_formula_a1("D1", "=LOOKUP(25,A1:C2)")
        .expect("LOOKUP");
    engine.set_formula_a1("D2", "=NA()").expect("NA");
    engine
        .set_formula_a1("D3", "=ERROR(\"boom\")")
        .expect("ERROR");

    assert_eq!(
        engine.cell_state_a1("D1").expect("D1").value,
        Value::Number(2.0)
    );
    assert!(matches!(
        engine.cell_state_a1("D2").expect("D2").value,
        Value::Error(_)
    ));
    assert!(matches!(
        engine.cell_state_a1("D3").expect("D3").value,
        Value::Error(_)
    ));
}

#[test]
fn evaluates_named_values_and_formulas() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 120.0).expect("A1");
    engine
        .set_name_formula("BASE_TOTAL", "=A1*1.1")
        .expect("base name");
    engine.set_name_number("TAX_RATE", 0.2).expect("tax name");
    engine
        .set_name_formula("GRAND_TOTAL", "=BASE_TOTAL*(1+TAX_RATE)")
        .expect("grand name");
    engine
        .set_formula_a1("B1", "=GRAND_TOTAL")
        .expect("B1 formula");

    match engine.cell_state_a1("B1").expect("B1").value {
        Value::Number(v) => assert!((v - 158.4).abs() < 1e-9),
        other => panic!("expected numeric B1, got {other:?}"),
    }
}

#[test]
fn reports_name_cycle_as_cell_error() {
    let mut engine = Engine::new();
    engine.set_name_formula("A", "=B+1").expect("set name A");
    engine.set_name_formula("B", "=A+1").expect("set name B");
    engine.set_formula_a1("C1", "=A").expect("set C1");

    let value = engine.cell_state_a1("C1").expect("query C1").value;
    match value {
        Value::Error(err) => assert!(err.to_string().contains("circular reference")),
        other => panic!("expected cycle error, got {other:?}"),
    }
}

#[test]
fn evaluates_let_bindings() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=LET(x,10,y,x*3,y+2)")
        .expect("set LET formula");
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(32.0)
    );
}

#[test]
fn evaluates_lambda_invocation_through_let() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=LET(inc,LAMBDA(v,v+1),inc(41))")
        .expect("set lambda formula");
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(42.0)
    );
}

#[test]
fn evaluates_direct_lambda_invocation() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=(LAMBDA(v,v+2))(40)")
        .expect("set direct lambda formula");
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(42.0)
    );
}

#[test]
fn evaluates_map_with_lambda_over_range() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine.set_number_a1("A2", 2.0).expect("A2");
    engine.set_number_a1("A3", 3.0).expect("A3");
    engine
        .set_formula_a1("B1", "=MAP(A1:A3,LAMBDA(x,x*10))")
        .expect("set MAP formula");

    assert_eq!(
        engine.cell_state_a1("B1").expect("B1").value,
        Value::Number(10.0)
    );
    assert_eq!(
        engine.cell_state_a1("B2").expect("B2").value,
        Value::Number(20.0)
    );
    assert_eq!(
        engine.cell_state_a1("B3").expect("B3").value,
        Value::Number(30.0)
    );
}

#[test]
fn evaluates_map_with_lambda_returning_arrays() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine.set_number_a1("A2", 2.0).expect("A2");
    engine
        .set_formula_a1("B1", "=MAP(A1:A2,LAMBDA(x,SEQUENCE(1,2,x,1)))")
        .expect("set MAP array-return formula");

    assert_eq!(
        engine.cell_state_a1("B1").expect("B1").value,
        Value::Number(1.0)
    );
    assert_eq!(
        engine.cell_state_a1("C1").expect("C1").value,
        Value::Number(2.0)
    );
    assert_eq!(
        engine.cell_state_a1("B2").expect("B2").value,
        Value::Number(2.0)
    );
    assert_eq!(
        engine.cell_state_a1("C2").expect("C2").value,
        Value::Number(3.0)
    );
}

#[test]
fn evaluates_row_and_column() {
    let mut engine = Engine::new();
    engine.set_formula_a1("C5", "=ROW()").expect("ROW");
    engine
        .set_formula_a1("D5", "=COLUMN(A1)")
        .expect("COLUMN(A1)");
    engine
        .set_formula_a1("E5", "=ROW(B8:B9)")
        .expect("ROW(range)");
    engine
        .set_formula_a1("F5", "=COLUMN(C9)")
        .expect("COLUMN(cell)");

    assert_eq!(
        engine.cell_state_a1("C5").expect("C5").value,
        Value::Number(5.0)
    );
    assert_eq!(
        engine.cell_state_a1("D5").expect("D5").value,
        Value::Number(1.0)
    );
    assert_eq!(
        engine.cell_state_a1("E5").expect("E5").value,
        Value::Number(8.0)
    );
    assert_eq!(
        engine.cell_state_a1("F5").expect("F5").value,
        Value::Number(3.0)
    );
}

#[test]
fn evaluates_indirect_and_offset() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_number_a1("B2", 25.0).expect("B2");
    engine.set_number_a1("B3", 5.0).expect("B3");
    engine
        .set_formula_a1("C1", "=INDIRECT(\"A1\")")
        .expect("INDIRECT");
    engine
        .set_formula_a1("C2", "=OFFSET(A1,1,1)")
        .expect("OFFSET scalar");
    engine
        .set_formula_a1("C3", "=SUM(OFFSET(A1,1,1,2,1))")
        .expect("OFFSET range");
    engine
        .set_formula_a1("D1", "=INDIRECT(\"R2C2\",FALSE)")
        .expect("INDIRECT R1C1 absolute");
    engine
        .set_formula_a1("E5", "=INDIRECT(\"R[-3]C[-3]\",FALSE)")
        .expect("INDIRECT R1C1 relative");
    engine
        .set_formula_a1("F1", "=SUM(INDIRECT(\"R2C2:R3C2\",FALSE))")
        .expect("INDIRECT R1C1 range");
    engine
        .set_formula_a1("G3", "=INDIRECT(\"RC[-5]\",FALSE)")
        .expect("INDIRECT R1C1 RC form");

    assert_eq!(
        engine.cell_state_a1("C1").expect("C1").value,
        Value::Number(10.0)
    );
    assert_eq!(
        engine.cell_state_a1("C2").expect("C2").value,
        Value::Number(25.0)
    );
    assert_eq!(
        engine.cell_state_a1("C3").expect("C3").value,
        Value::Number(30.0)
    );
    assert_eq!(
        engine.cell_state_a1("D1").expect("D1").value,
        Value::Number(25.0)
    );
    assert_eq!(
        engine.cell_state_a1("E5").expect("E5").value,
        Value::Number(25.0)
    );
    assert_eq!(
        engine.cell_state_a1("F1").expect("F1").value,
        Value::Number(30.0)
    );
    assert_eq!(
        engine.cell_state_a1("G3").expect("G3").value,
        Value::Number(5.0)
    );
}
