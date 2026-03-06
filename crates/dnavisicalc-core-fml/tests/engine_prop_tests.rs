use dnavisicalc_core::{CellInput, Engine, RecalcMode, Value};
use proptest::prelude::*;

fn finite_num() -> impl Strategy<Value = f64> {
    (-1_000_000f64..1_000_000f64).prop_filter("finite", |v| v.is_finite())
}

proptest! {
    #[test]
    fn manual_mode_requires_explicit_recalc(a in finite_num(), b in finite_num()) {
        let mut engine = Engine::new();
        engine.set_recalc_mode(RecalcMode::Manual);

        engine.set_number_a1("A1", a).expect("set A1");
        engine.set_number_a1("B1", b).expect("set B1");
        engine.set_formula_a1("C1", "=A1+B1").expect("set C1");

        let state_before = engine.cell_state_a1("C1").expect("query C1");
        prop_assert!(state_before.stale);

        engine.recalculate().expect("recalc");
        let state_after = engine.cell_state_a1("C1").expect("query C1 after recalc");
        prop_assert!(!state_after.stale);

        match state_after.value {
            Value::Number(n) => prop_assert!((n - (a + b)).abs() < 1e-9),
            other => prop_assert!(false, "expected numeric result, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_result_independent_of_set_order(a in finite_num(), b in finite_num()) {
        let mut first = Engine::new();
        first.set_number_a1("A1", a).expect("set A1");
        first.set_number_a1("B1", b).expect("set B1");
        first.set_formula_a1("C1", "=A1+B1").expect("set C1");

        let mut second = Engine::new();
        second.set_formula_a1("C1", "=A1+B1").expect("set C1");
        second.set_number_a1("B1", b).expect("set B1");
        second.set_number_a1("A1", a).expect("set A1");

        let v1 = first.cell_state_a1("C1").expect("query first").value;
        let v2 = second.cell_state_a1("C1").expect("query second").value;
        prop_assert_eq!(v1, v2);
    }
}

#[test]
fn all_cell_inputs_is_sorted_for_stable_serialization() {
    let mut engine = Engine::new();
    engine
        .set_cell_input_a1("B2", CellInput::Number(1.0))
        .expect("set B2");
    engine
        .set_cell_input_a1("A1", CellInput::Number(2.0))
        .expect("set A1");

    let cells = engine.all_cell_inputs();
    let labels = cells
        .into_iter()
        .map(|(cell, _)| cell.to_string())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["A1".to_string(), "B2".to_string()]);
}
