use dnavisicalc_core::{Engine, RecalcMode, Value};

fn assert_number(value: &Value, expected: f64) {
    match value {
        Value::Number(actual) => assert!(
            (actual - expected).abs() < 1e-9,
            "expected number {expected}, got {actual}"
        ),
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

#[test]
fn seam_static_no_dependency_formula() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "SIN(0)").expect("set formula");
    assert_number(&engine.cell_state_a1("A1").expect("A1").value, 0.0);
}

#[test]
fn seam_static_ref_only_formula() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 2.0).expect("A1");
    engine.set_formula_a1("B1", "=A1+1").expect("B1");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 3.0);

    engine.set_number_a1("A1", 5.0).expect("A1 update");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 6.0);
    assert_eq!(engine.last_eval_count(), 1);
}

#[test]
fn seam_name_chain_path() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 120.0).expect("A1");
    engine
        .set_name_formula("BASE_TOTAL", "=A1*1.1")
        .expect("BASE_TOTAL");
    engine.set_name_number("TAX_RATE", 0.2).expect("TAX_RATE");
    engine
        .set_name_formula("GRAND_TOTAL", "=BASE_TOTAL*(1+TAX_RATE)")
        .expect("GRAND_TOTAL");
    engine.set_formula_a1("B1", "=GRAND_TOTAL").expect("B1");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 158.4);
}

#[test]
fn seam_branch_flip_behavior() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine.set_number_a1("A2", 10.0).expect("A2");
    engine.set_number_a1("A3", 20.0).expect("A3");
    engine
        .set_formula_a1("B1", "=IF(A1>0,A2,A3)")
        .expect("B1 formula");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 10.0);

    engine.set_number_a1("A1", -1.0).expect("flip branch");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 20.0);
    assert_eq!(engine.last_eval_count(), 1);
}

#[test]
fn seam_dynamic_intent_reference_functions() {
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

    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 10.0);
    assert_number(&engine.cell_state_a1("C2").expect("C2").value, 25.0);
    assert_number(&engine.cell_state_a1("C3").expect("C3").value, 30.0);
}

#[test]
fn seam_spill_shape_change_updates_hash_consumer() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2,1,1)")
        .expect("A1 sequence");
    engine.set_formula_a1("C1", "=SUM(A1#)").expect("C1 sum");
    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 10.0);

    engine
        .set_formula_a1("A1", "=SEQUENCE(3,1,10,1)")
        .expect("A1 reshape");
    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 33.0);
}

#[test]
fn seam_volatile_and_external_paths() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=NOW()").expect("NOW");
    engine.set_formula_a1("B1", "=RAND()").expect("RAND");
    engine.set_formula_a1("C1", "=STREAM(1)").expect("STREAM");

    assert!(engine.has_volatile_cells());
    assert!(engine.has_externally_invalidated_cells());
    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 0.0);

    assert!(engine.tick_streams(1.2));
    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 1.0);
}

#[test]
fn seam_structural_edit_refreshes_formula_position_and_value() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_number_a1("B1", 20.0).expect("B1");
    engine.set_formula_a1("C1", "=A1+B1").expect("C1");

    engine.insert_col(1).expect("insert col A");

    let source = engine
        .formula_source_a1("D1")
        .expect("formula source")
        .expect("formula exists");
    assert!(source.contains("B1"));
    assert!(source.contains("C1"));
    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 30.0);
}

#[test]
fn seam_manual_vs_automatic_recalc_modes() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine.set_formula_a1("B1", "=A1+1").expect("B1");

    let stale_state = engine.cell_state_a1("B1").expect("B1 stale");
    assert!(stale_state.stale);

    engine.recalculate().expect("manual recalc");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 2.0);

    engine.set_recalc_mode(RecalcMode::Automatic);
    engine.set_number_a1("A1", 5.0).expect("A1 update");
    let fresh_state = engine.cell_state_a1("B1").expect("B1 fresh");
    assert!(!fresh_state.stale);
    assert_number(&fresh_state.value, 6.0);
}

#[test]
fn seam_incremental_dirty_closure_skips_unrelated_formula() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("A1");
    engine.set_number_a1("A2", 5.0).expect("A2");
    engine.set_formula_a1("B1", "=A1+1").expect("B1");
    engine.set_formula_a1("C1", "=B1+1").expect("C1");
    engine.set_formula_a1("D1", "=A2+1").expect("D1");

    engine.set_number_a1("A1", 10.0).expect("A1 update");

    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 11.0);
    assert_number(&engine.cell_state_a1("C1").expect("C1").value, 12.0);
    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 6.0);
    assert_eq!(engine.last_eval_count(), 2);
}

#[test]
fn seam_dynamic_reference_retargeting_flows() {
    let mut engine = Engine::new();

    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_number_a1("A2", 20.0).expect("A2");
    engine.set_number_a1("B1", 100.0).expect("B1");
    engine.set_number_a1("B2", 200.0).expect("B2");
    engine.set_number_a1("C1", 0.0).expect("C1 selector");
    engine.set_number_a1("C2", 0.0).expect("C2 selector");

    engine
        .set_formula_a1("D1", "=INDIRECT(IF(C1=0,\"A1\",\"A2\"))")
        .expect("D1 INDIRECT");
    engine
        .set_formula_a1("D2", "=OFFSET(B1,C2,0)")
        .expect("D2 OFFSET");

    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 10.0);
    assert_number(&engine.cell_state_a1("D2").expect("D2").value, 100.0);

    engine.set_number_a1("A1", 11.0).expect("A1 update");
    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 11.0);

    engine
        .set_number_a1("C1", 1.0)
        .expect("flip INDIRECT selector");
    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 20.0);

    engine.set_number_a1("A2", 21.0).expect("A2 update");
    assert_number(&engine.cell_state_a1("D1").expect("D1").value, 21.0);

    // OFFSET keeps static dependency on anchor B1.
    engine.set_number_a1("B1", 101.0).expect("B1 update");
    assert_number(&engine.cell_state_a1("D2").expect("D2").value, 101.0);

    engine
        .set_number_a1("C2", 1.0)
        .expect("flip OFFSET selector");
    assert_number(&engine.cell_state_a1("D2").expect("D2").value, 200.0);

    engine.set_number_a1("B2", 205.0).expect("B2 update");
    assert_number(&engine.cell_state_a1("D2").expect("D2").value, 205.0);
}

#[test]
fn seam_spill_takeover_and_clearance_on_referenced_spill_child() {
    let mut engine = Engine::new();

    engine
        .set_formula_a1("A1", "=SEQUENCE(1,1,9,1)")
        .expect("A1 single");
    engine
        .set_formula_a1("B1", "=IF(ISBLANK(A2),-1,A2*10)")
        .expect("B1 references potential spill child");
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, -1.0);

    // Spill expands to include A2; B1 should update from blank-path to spill child value.
    engine
        .set_formula_a1("A1", "=SEQUENCE(3,1,1,1)")
        .expect("A1 expanded spill");
    assert_number(&engine.cell_state_a1("A2").expect("A2").value, 2.0);
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, 20.0);

    // Spill shrinks away from A2; B1 should observe spill clearance and return to blank-path.
    engine
        .set_formula_a1("A1", "=SEQUENCE(1,1,9,1)")
        .expect("A1 shrunk spill");
    assert_eq!(engine.cell_state_a1("A2").expect("A2").value, Value::Blank);
    assert_number(&engine.cell_state_a1("B1").expect("B1").value, -1.0);
}
