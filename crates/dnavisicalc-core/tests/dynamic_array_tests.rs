use dnavisicalc_core::{CellError, CellRef, Engine, Value};

#[test]
fn sequence_spills_and_marks_spill_metadata() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(3,2,1,1)")
        .expect("set formula");

    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(1.0)
    );
    assert_eq!(
        engine.cell_state_a1("B1").expect("B1").value,
        Value::Number(2.0)
    );
    assert_eq!(
        engine.cell_state_a1("A3").expect("A3").value,
        Value::Number(5.0)
    );
    assert_eq!(
        engine
            .spill_anchor_for_cell_a1("B2")
            .expect("spill anchor query"),
        Some(CellRef::from_a1("A1").expect("A1"))
    );
    assert_eq!(
        engine
            .spill_range_for_anchor(CellRef::from_a1("A1").expect("A1"))
            .expect("spill range"),
        Some(dnavisicalc_core::CellRange::new(
            CellRef::from_a1("A1").expect("A1"),
            CellRef::from_a1("B3").expect("B3")
        ))
    );
}

#[test]
fn blocked_spill_returns_spill_error() {
    let mut engine = Engine::new();
    engine.set_number_a1("A2", 99.0).expect("set number");
    engine
        .set_formula_a1("A1", "=SEQUENCE(3)")
        .expect("set formula");

    match engine.cell_state_a1("A1").expect("A1").value {
        Value::Error(CellError::Spill(msg)) => assert!(msg.contains("blocked")),
        other => panic!("expected spill error, got {other:?}"),
    }
    assert_eq!(
        engine.cell_state_a1("A2").expect("A2").value,
        Value::Number(99.0)
    );
}

#[test]
fn spill_ref_works_for_spilled_anchor() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2,1,1)")
        .expect("set sequence");
    engine.set_formula_a1("C1", "=SUM(A1#)").expect("set sum");

    assert_eq!(
        engine.cell_state_a1("C1").expect("C1").value,
        Value::Number(10.0)
    );
}

#[test]
fn spill_ref_errors_when_anchor_not_spilled() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 1.0).expect("set number");
    engine
        .set_formula_a1("B1", "=SUM(A1#)")
        .expect("set formula");

    match engine.cell_state_a1("B1").expect("B1").value {
        Value::Error(CellError::Ref(msg)) => assert!(msg.contains("does not contain a spilled")),
        other => panic!("expected ref error, got {other:?}"),
    }
}

#[test]
fn binary_expression_arrayifies_over_spill_reference() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2)")
        .expect("set sequence");
    engine
        .set_formula_a1("C1", "=A1# + 10")
        .expect("set formula");

    assert_eq!(
        engine.cell_state_a1("C1").expect("C1").value,
        Value::Number(11.0)
    );
    assert_eq!(
        engine.cell_state_a1("D2").expect("D2").value,
        Value::Number(14.0)
    );
}

#[test]
fn randarray_spills_within_bounds() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=RANDARRAY(2,3,5,7,TRUE)")
        .expect("set formula");

    for cell in ["A1", "B1", "C1", "A2", "B2", "C2"] {
        let value = engine.cell_state_a1(cell).expect("cell state").value;
        match value {
            Value::Number(n) => {
                assert!((5.0..=7.0).contains(&n));
                assert_eq!(n.fract(), 0.0);
            }
            other => panic!("expected number in {cell}, got {other:?}"),
        }
    }
}

#[test]
fn direct_reference_to_spilled_interior_cell_works() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2,1,1)")
        .expect("set sequence");
    engine.set_formula_a1("D1", "=B2").expect("set ref formula");

    assert_eq!(
        engine.cell_state_a1("D1").expect("D1").value,
        Value::Number(4.0)
    );
}

#[test]
fn range_aggregate_over_spilled_interior_cells_works() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2,1,1)")
        .expect("set sequence");
    engine
        .set_formula_a1("D1", "=SUM(A1:B2)")
        .expect("set sum formula");

    assert_eq!(
        engine.cell_state_a1("D1").expect("D1").value,
        Value::Number(10.0)
    );
}
