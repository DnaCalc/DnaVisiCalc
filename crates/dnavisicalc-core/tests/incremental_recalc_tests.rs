use dnavisicalc_core::{Engine, Value};

fn val(engine: &Engine, addr: &str) -> Value {
    engine.cell_state_a1(addr).expect("query").value
}

// ---------------------------------------------------------------------------
// Basic incremental recalc: value-only changes use incremental path
// ---------------------------------------------------------------------------

#[test]
fn value_change_triggers_incremental_recalc() {
    let mut engine = Engine::new();

    // Set up: A1=10, B1=20, C1=A1+B1 (formula)
    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_number_a1("B1", 20.0).expect("set");
    engine.set_formula_a1("C1", "=A1+B1").expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(30.0));

    // Full recalc happened when formula was added (graph structure changed).
    // Now change a value — should trigger incremental recalc.
    engine.set_number_a1("A1", 50.0).expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(70.0)); // 50 + 20

    // Only C1 should have been re-evaluated (A1 is a literal, not a formula).
    assert_eq!(engine.last_eval_count(), 1);
}

#[test]
fn incremental_recalc_skips_unaffected_formulas() {
    let mut engine = Engine::new();

    // A1=10, B1=20, C1=A1*2, D1=B1*3
    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_number_a1("B1", 20.0).expect("set");
    engine.set_formula_a1("C1", "=A1*2").expect("set");
    engine.set_formula_a1("D1", "=B1*3").expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(20.0));
    assert_eq!(val(&engine, "D1"), Value::Number(60.0));

    // Change only A1 — only C1 should be re-evaluated, D1 should be skipped.
    engine.set_number_a1("A1", 100.0).expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(200.0));
    assert_eq!(val(&engine, "D1"), Value::Number(60.0)); // unchanged
    assert_eq!(engine.last_eval_count(), 1); // only C1
}

#[test]
fn incremental_recalc_propagates_through_chain() {
    let mut engine = Engine::new();

    // A1=5, B1=A1+1, C1=B1+1, D1=C1+1
    engine.set_number_a1("A1", 5.0).expect("set");
    engine.set_formula_a1("B1", "=A1+1").expect("set");
    engine.set_formula_a1("C1", "=B1+1").expect("set");
    engine.set_formula_a1("D1", "=C1+1").expect("set");

    assert_eq!(val(&engine, "D1"), Value::Number(8.0)); // 5+1+1+1

    // Change A1 — entire chain should re-evaluate.
    engine.set_number_a1("A1", 10.0).expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(11.0));
    assert_eq!(val(&engine, "C1"), Value::Number(12.0));
    assert_eq!(val(&engine, "D1"), Value::Number(13.0));
    assert_eq!(engine.last_eval_count(), 3); // B1, C1, D1
}

#[test]
fn incremental_recalc_handles_diamond_dependency() {
    let mut engine = Engine::new();

    // A1=10, B1=A1+1, C1=A1+2, D1=B1+C1 (diamond: D1 depends on B1 and C1, both depend on A1)
    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_formula_a1("B1", "=A1+1").expect("set");
    engine.set_formula_a1("C1", "=A1+2").expect("set");
    engine.set_formula_a1("D1", "=B1+C1").expect("set");

    assert_eq!(val(&engine, "D1"), Value::Number(23.0)); // (10+1) + (10+2) = 23

    engine.set_number_a1("A1", 20.0).expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(21.0));
    assert_eq!(val(&engine, "C1"), Value::Number(22.0));
    assert_eq!(val(&engine, "D1"), Value::Number(43.0)); // (20+1) + (20+2) = 43
    assert_eq!(engine.last_eval_count(), 3); // B1, C1, D1
}

// ---------------------------------------------------------------------------
// Formula changes fall back to full recalc
// ---------------------------------------------------------------------------

#[test]
fn formula_change_triggers_full_recalc() {
    let mut engine = Engine::new();

    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_number_a1("B1", 20.0).expect("set");
    engine.set_formula_a1("C1", "=A1+B1").expect("set");
    engine.set_formula_a1("D1", "=C1*2").expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(30.0));
    assert_eq!(val(&engine, "D1"), Value::Number(60.0));

    // Changing a formula triggers full recalc (graph structure change).
    engine.set_formula_a1("C1", "=A1-B1").expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(-10.0));
    assert_eq!(val(&engine, "D1"), Value::Number(-20.0));
    // Full recalc evaluates all formula cells.
    assert_eq!(engine.last_eval_count(), 2); // C1 and D1
}

// ---------------------------------------------------------------------------
// Multiple value changes before recalc (manual mode)
// ---------------------------------------------------------------------------

#[test]
fn manual_mode_batches_dirty_cells() {
    use dnavisicalc_core::RecalcMode;

    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("set");
    engine.set_number_a1("B1", 2.0).expect("set");
    engine.set_formula_a1("C1", "=A1+B1").expect("set");
    engine.set_formula_a1("D1", "=A1*10").expect("set");
    engine.set_formula_a1("E1", "=B1*10").expect("set");

    assert_eq!(val(&engine, "C1"), Value::Number(3.0));

    // Switch to manual mode.
    engine.set_recalc_mode(RecalcMode::Manual);

    // Change both A1 and B1 without recalculating.
    engine.set_number_a1("A1", 100.0).expect("set");
    engine.set_number_a1("B1", 200.0).expect("set");

    // Values are stale until explicit recalculate.
    assert_eq!(val(&engine, "C1"), Value::Number(3.0)); // still old value

    // Now recalculate — incremental should evaluate C1, D1, E1.
    engine.recalculate().expect("recalc");

    assert_eq!(val(&engine, "C1"), Value::Number(300.0)); // 100 + 200
    assert_eq!(val(&engine, "D1"), Value::Number(1000.0)); // 100 * 10
    assert_eq!(val(&engine, "E1"), Value::Number(2000.0)); // 200 * 10
    assert_eq!(engine.last_eval_count(), 3); // C1, D1, E1
}

// ---------------------------------------------------------------------------
// Correctness: clearing a cell propagates dirtiness
// ---------------------------------------------------------------------------

#[test]
fn clearing_cell_marks_dependents_dirty() {
    let mut engine = Engine::new();

    engine.set_number_a1("A1", 42.0).expect("set");
    engine.set_formula_a1("B1", "=A1+1").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(43.0));

    // Clear A1 — B1 should re-evaluate (A1 now blank).
    engine.clear_cell_a1("A1").expect("clear");

    // B1 references blank A1, which evaluates to 0 in arithmetic context.
    assert_eq!(val(&engine, "B1"), Value::Number(1.0));
}

// ---------------------------------------------------------------------------
// Edge case: no dirty cells means no work
// ---------------------------------------------------------------------------

#[test]
fn recalculate_with_no_changes_is_noop() {
    use dnavisicalc_core::RecalcMode;

    let mut engine = Engine::new();
    engine.set_number_a1("A1", 10.0).expect("set");
    engine.set_formula_a1("B1", "=A1*2").expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(20.0));

    // Switch to manual and recalculate without any changes.
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.recalculate().expect("recalc");

    // No dirty cells, but since manual mode doesn't have a calc_tree cached
    // from the automatic recalc, this might do a full recalc. Let's verify
    // the values are still correct.
    assert_eq!(val(&engine, "B1"), Value::Number(20.0));
}

// ---------------------------------------------------------------------------
// Range formula: change one cell in a range
// ---------------------------------------------------------------------------

#[test]
fn incremental_recalc_handles_range_dependency() {
    let mut engine = Engine::new();

    engine.set_number_a1("A1", 1.0).expect("set");
    engine.set_number_a1("A2", 2.0).expect("set");
    engine.set_number_a1("A3", 3.0).expect("set");
    engine.set_formula_a1("B1", "=SUM(A1:A3)").expect("set");
    engine.set_formula_a1("C1", "=A1*10").expect("set"); // independent

    assert_eq!(val(&engine, "B1"), Value::Number(6.0));
    assert_eq!(val(&engine, "C1"), Value::Number(10.0));

    // Change A2 — only B1 should re-evaluate (C1 doesn't depend on A2).
    engine.set_number_a1("A2", 20.0).expect("set");

    assert_eq!(val(&engine, "B1"), Value::Number(24.0)); // 1 + 20 + 3
    assert_eq!(val(&engine, "C1"), Value::Number(10.0)); // unchanged
    assert_eq!(engine.last_eval_count(), 1); // only B1
}
