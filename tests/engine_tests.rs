use dnavisicalc_core::{Engine, EngineError, RecalcMode, Value};

#[test]
fn manual_mode_marks_values_as_stale_until_recalc() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 2.0).expect("set A1");
    engine
        .set_formula_a1("B1", "=A1*2")
        .expect("set B1 formula");

    let before = engine.cell_state_a1("B1").expect("query B1");
    assert!(before.stale);
    assert_eq!(before.value, Value::Blank);

    engine.recalculate().expect("manual recalc");
    let after = engine.cell_state_a1("B1").expect("query B1 after recalc");
    assert!(!after.stale);
    assert_eq!(after.value, Value::Number(4.0));
    assert_eq!(engine.committed_epoch(), engine.stabilized_epoch());
}

#[test]
fn automatic_mode_recalculates_on_edit() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 3.0).expect("set A1");
    engine
        .set_formula_a1("B1", "=A1^2")
        .expect("set B1 formula");
    let initial = engine.cell_state_a1("B1").expect("query B1");
    assert_eq!(initial.value, Value::Number(9.0));
    assert!(!initial.stale);

    engine.set_number_a1("A1", 4.0).expect("update A1");
    let updated = engine.cell_state_a1("B1").expect("query B1 updated");
    assert_eq!(updated.value, Value::Number(16.0));
    assert!(!updated.stale);
}

#[test]
fn rejects_circular_dependencies() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine
        .set_formula_a1("A1", "=B1+1")
        .expect("set A1 formula");
    engine
        .set_formula_a1("B1", "=A1+1")
        .expect("set B1 formula");

    let err = engine.recalculate().expect_err("recalc should fail");
    match err {
        EngineError::Dependency(dep_err) => {
            let msg = dep_err.to_string();
            assert!(msg.contains("circular reference"));
            assert!(msg.contains("A1"));
            assert!(msg.contains("B1"));
        }
        _ => panic!("expected dependency error"),
    }
}

#[test]
fn enforces_visicalc_bounds() {
    let mut engine = Engine::new();
    engine
        .set_number_a1("BK254", 1.0)
        .expect("BK254 should be in range");
    let err = engine
        .set_number_a1("BL1", 1.0)
        .expect_err("BL1 should be out of range");
    match err {
        EngineError::Address(address_err) => {
            assert!(address_err.to_string().contains("out of bounds"));
        }
        _ => panic!("expected address error"),
    }
}
