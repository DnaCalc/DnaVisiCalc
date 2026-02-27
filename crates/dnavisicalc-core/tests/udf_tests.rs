use dnavisicalc_core::{Engine, FnUdf, Value};

fn val(engine: &Engine, addr: &str) -> Value {
    engine.cell_state_a1(addr).expect("query").value
}

// ---------------------------------------------------------------------------
// Basic UDF registration and invocation
// ---------------------------------------------------------------------------

#[test]
fn udf_basic_invocation() {
    let mut engine = Engine::new();

    // Register a UDF that doubles a number.
    engine.register_udf(
        "DOUBLE",
        Box::new(FnUdf(|args: &[Value]| -> Value {
            match args.first() {
                Some(Value::Number(n)) => Value::Number(n * 2.0),
                _ => Value::Error(dnavisicalc_core::CellError::Value(
                    "expected number".to_string(),
                )),
            }
        })),
    );

    engine.set_number_a1("A1", 21.0).expect("set");
    engine.set_formula_a1("B1", "=DOUBLE(A1)").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(42.0));
}

#[test]
fn udf_case_insensitive() {
    let mut engine = Engine::new();

    engine.register_udf(
        "myFunc",
        Box::new(FnUdf(|args: &[Value]| -> Value {
            match args.first() {
                Some(Value::Number(n)) => Value::Number(n + 100.0),
                _ => Value::Number(100.0),
            }
        })),
    );

    engine.set_number_a1("A1", 5.0).expect("set");
    // Formula parser uppercases function names, so this should match.
    engine.set_formula_a1("B1", "=MYFUNC(A1)").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(105.0));
}

#[test]
fn udf_multiple_arguments() {
    let mut engine = Engine::new();

    // Register a UDF that sums all numeric arguments.
    engine.register_udf(
        "MYSUM",
        Box::new(FnUdf(|args: &[Value]| -> Value {
            let total: f64 = args
                .iter()
                .filter_map(|v| match v {
                    Value::Number(n) => Some(*n),
                    _ => None,
                })
                .sum();
            Value::Number(total)
        })),
    );

    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_number_a1("B1", 20.0).expect("set");
    engine.set_number_a1("C1", 30.0).expect("set");
    engine
        .set_formula_a1("D1", "=MYSUM(A1,B1,C1)")
        .expect("set");

    assert_eq!(val(&engine, "D1"), Value::Number(60.0));
}

#[test]
fn udf_no_arguments() {
    let mut engine = Engine::new();

    engine.register_udf(
        "FORTYTWO",
        Box::new(FnUdf(|_args: &[Value]| -> Value { Value::Number(42.0) })),
    );

    engine.set_formula_a1("A1", "=FORTYTWO()").expect("set");
    assert_eq!(val(&engine, "A1"), Value::Number(42.0));
}

#[test]
fn udf_returning_text() {
    let mut engine = Engine::new();

    engine.register_udf(
        "GREET",
        Box::new(FnUdf(|args: &[Value]| -> Value {
            match args.first() {
                Some(Value::Text(name)) => Value::Text(format!("Hello, {}!", name)),
                _ => Value::Text("Hello!".to_string()),
            }
        })),
    );

    engine.set_text_a1("A1", "World").expect("set");
    engine.set_formula_a1("B1", "=GREET(A1)").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Text("Hello, World!".to_string()));
}

#[test]
fn udf_returning_error() {
    let mut engine = Engine::new();

    engine.register_udf(
        "FAILME",
        Box::new(FnUdf(|_args: &[Value]| -> Value {
            Value::Error(dnavisicalc_core::CellError::Value(
                "intentional error".to_string(),
            ))
        })),
    );

    engine.set_formula_a1("A1", "=FAILME()").expect("set");

    match val(&engine, "A1") {
        Value::Error(dnavisicalc_core::CellError::Value(msg)) => {
            assert_eq!(msg, "intentional error");
        }
        other => panic!("expected error, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// UDF unregistration
// ---------------------------------------------------------------------------

#[test]
fn udf_unregister() {
    let mut engine = Engine::new();

    engine.register_udf("TEMP", Box::new(FnUdf(|_: &[Value]| Value::Number(1.0))));

    engine.set_formula_a1("A1", "=TEMP()").expect("set");
    assert_eq!(val(&engine, "A1"), Value::Number(1.0));

    // Unregister and re-evaluate — should now produce #NAME? error.
    assert!(engine.unregister_udf("TEMP"));
    engine
        .set_formula_a1("A1", "=TEMP()")
        .expect("set again to trigger recalc");

    match val(&engine, "A1") {
        Value::Error(dnavisicalc_core::CellError::Name(_)) => {} // expected
        other => panic!("expected #NAME? error, got {:?}", other),
    }
}

#[test]
fn udf_unregister_nonexistent_returns_false() {
    let mut engine = Engine::new();
    assert!(!engine.unregister_udf("NONEXISTENT"));
}

// ---------------------------------------------------------------------------
// UDF replacement
// ---------------------------------------------------------------------------

#[test]
fn udf_replacement() {
    let mut engine = Engine::new();

    engine.register_udf("CUSTOM", Box::new(FnUdf(|_: &[Value]| Value::Number(1.0))));

    engine.set_formula_a1("A1", "=CUSTOM()").expect("set");
    assert_eq!(val(&engine, "A1"), Value::Number(1.0));

    // Replace with a new implementation.
    engine.register_udf("CUSTOM", Box::new(FnUdf(|_: &[Value]| Value::Number(99.0))));

    // Re-set formula to force recalc.
    engine.set_formula_a1("A1", "=CUSTOM()").expect("set");
    assert_eq!(val(&engine, "A1"), Value::Number(99.0));
}

// ---------------------------------------------------------------------------
// UDF does not shadow built-in functions
// ---------------------------------------------------------------------------

#[test]
fn udf_does_not_shadow_builtins() {
    let mut engine = Engine::new();

    // SUM is a built-in. Register a UDF with the same name.
    engine.register_udf("SUM", Box::new(FnUdf(|_: &[Value]| Value::Number(999.0))));

    engine.set_number_a1("A1", 1.0).expect("set");
    engine.set_number_a1("A2", 2.0).expect("set");
    engine.set_formula_a1("B1", "=SUM(A1:A2)").expect("set");

    // Built-in SUM should take precedence, not the UDF.
    assert_eq!(val(&engine, "B1"), Value::Number(3.0));
}

// ---------------------------------------------------------------------------
// UDF interacts correctly with incremental recalc
// ---------------------------------------------------------------------------

#[test]
fn udf_with_incremental_recalc() {
    let mut engine = Engine::new();

    engine.register_udf(
        "TRIPLE",
        Box::new(FnUdf(|args: &[Value]| -> Value {
            match args.first() {
                Some(Value::Number(n)) => Value::Number(n * 3.0),
                _ => Value::Number(0.0),
            }
        })),
    );

    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_formula_a1("B1", "=TRIPLE(A1)").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(30.0));

    // Change A1 — incremental recalc should re-evaluate B1 with the UDF.
    engine.set_number_a1("A1", 20.0).expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(60.0));
    assert_eq!(engine.last_eval_count(), 1);
}
