use dnavisicalc_core::{CellError, Engine, Value};

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
fn indirect_r1c1_relative_name_reference_uses_non_iterative_cycle_fallback() {
    let mut engine = Engine::new();
    engine
        .set_name_formula("REL_REF", "=INDIRECT(\"RC\",FALSE)")
        .expect("set name formula");
    engine
        .set_formula_a1("A1", "=REL_REF")
        .expect("set formula");

    let value = engine.cell_state_a1("A1").expect("query").value;
    assert_eq!(value, Value::Number(0.0));
}

// --- IFERROR ---

#[test]
fn iferror_returns_value_when_no_error() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine
        .set_formula_a1("B1", "=IFERROR(A1, 0)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert_eq!(value, Value::Number(10.0));
}

#[test]
fn iferror_returns_fallback_when_error() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=1/0").expect("set formula");
    engine
        .set_formula_a1("B1", "=IFERROR(A1, -1)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert_eq!(value, Value::Number(-1.0));
}

#[test]
fn iferror_catches_na_error() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=NA()").expect("set formula");
    engine
        .set_formula_a1("B1", "=IFERROR(A1, 42)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert_eq!(value, Value::Number(42.0));
}

#[test]
fn iferror_arity_error() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=IFERROR(1)")
        .expect("set formula");
    assert!(matches!(
        engine.cell_state_a1("A1").expect("query").value,
        Value::Error(_)
    ));
}

// --- IFNA ---

#[test]
fn ifna_returns_value_when_no_error() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 5.0).expect("A1");
    engine
        .set_formula_a1("B1", "=IFNA(A1, 0)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert_eq!(value, Value::Number(5.0));
}

#[test]
fn ifna_returns_fallback_only_for_na() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=NA()").expect("set formula");
    engine
        .set_formula_a1("B1", "=IFNA(A1, 99)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert_eq!(value, Value::Number(99.0));
}

#[test]
fn ifna_does_not_catch_div_by_zero() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=1/0").expect("set formula");
    engine
        .set_formula_a1("B1", "=IFNA(A1, 0)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert!(matches!(value, Value::Error(CellError::DivisionByZero)));
}

// --- ISERROR ---

#[test]
fn iserror_true_for_error() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=1/0").expect("set formula");
    engine
        .set_formula_a1("B1", "=ISERROR(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn iserror_false_for_number() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 42.0).expect("A1");
    engine
        .set_formula_a1("B1", "=ISERROR(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ISNA ---

#[test]
fn isna_true_for_na() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=NA()").expect("set formula");
    engine
        .set_formula_a1("B1", "=ISNA(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn isna_false_for_other_error() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=1/0").expect("set formula");
    engine
        .set_formula_a1("B1", "=ISNA(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ISBLANK ---

#[test]
fn isblank_true_for_empty_cell() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("B1", "=ISBLANK(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn isblank_false_for_number() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 0.0).expect("A1");
    engine
        .set_formula_a1("B1", "=ISBLANK(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ISTEXT ---

#[test]
fn istext_true_for_text() {
    let mut engine = Engine::new();
    engine.set_text_a1("A1", "hello").expect("A1");
    engine
        .set_formula_a1("B1", "=ISTEXT(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn istext_false_for_number() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine
        .set_formula_a1("B1", "=ISTEXT(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ISNUMBER ---

#[test]
fn isnumber_true_for_number() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 3.14).expect("A1");
    engine
        .set_formula_a1("B1", "=ISNUMBER(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn isnumber_false_for_text() {
    let mut engine = Engine::new();
    engine.set_text_a1("A1", "abc").expect("A1");
    engine
        .set_formula_a1("B1", "=ISNUMBER(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ISLOGICAL ---

#[test]
fn islogical_true_for_bool() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=TRUE").expect("set formula");
    engine
        .set_formula_a1("B1", "=ISLOGICAL(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(true)
    );
}

#[test]
fn islogical_false_for_number() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine
        .set_formula_a1("B1", "=ISLOGICAL(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Bool(false)
    );
}

// --- ERROR.TYPE ---

#[test]
fn error_type_returns_correct_numbers() {
    let mut engine = Engine::new();
    // #DIV/0! = 2
    engine.set_formula_a1("A1", "=1/0").expect("A1");
    engine
        .set_formula_a1("B1", "=ERROR.TYPE(A1)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Number(2.0)
    );

    // #N/A = 7
    engine.set_formula_a1("A2", "=NA()").expect("A2");
    engine
        .set_formula_a1("B2", "=ERROR.TYPE(A2)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B2").expect("query").value,
        Value::Number(7.0)
    );

    // #NAME? = 5
    engine.set_formula_a1("A3", "=BOGUS(1)").expect("A3");
    engine
        .set_formula_a1("B3", "=ERROR.TYPE(A3)")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("B3").expect("query").value,
        Value::Number(5.0)
    );
}

#[test]
fn error_type_returns_na_for_non_error() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 42.0).expect("A1");
    engine
        .set_formula_a1("B1", "=ERROR.TYPE(A1)")
        .expect("set formula");
    let value = engine.cell_state_a1("B1").expect("query").value;
    assert!(matches!(value, Value::Error(CellError::Na)));
}

// --- NA() produces proper Na variant ---

#[test]
fn na_function_produces_na_variant() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=NA()").expect("set formula");
    let value = engine.cell_state_a1("A1").expect("query").value;
    assert!(matches!(value, Value::Error(CellError::Na)));
}

// --- CellError::excel_tag() ---

#[test]
fn cell_error_excel_tags_are_correct() {
    assert_eq!(CellError::DivisionByZero.excel_tag(), "#DIV/0!");
    assert_eq!(CellError::Na.excel_tag(), "#N/A");
    assert_eq!(CellError::Null.excel_tag(), "#NULL!");
    assert_eq!(CellError::Num("overflow".to_string()).excel_tag(), "#NUM!");
    assert_eq!(CellError::Value("test".to_string()).excel_tag(), "#VALUE!");
    assert_eq!(CellError::Name("BOGUS".to_string()).excel_tag(), "#NAME?");
    assert_eq!(CellError::Ref("deleted".to_string()).excel_tag(), "#REF!");
    assert_eq!(
        CellError::Spill("blocked".to_string()).excel_tag(),
        "#SPILL!"
    );
}
