use dnavisicalc_core::{CellError, CellRange, CellRef, DynamicArrayStrategy, Engine, Value};

fn strategies() -> [DynamicArrayStrategy; 3] {
    [
        DynamicArrayStrategy::OverlayInline,
        DynamicArrayStrategy::OverlayPlanner,
        DynamicArrayStrategy::RewriteMaterialize,
    ]
}

fn engine_with_strategy(strategy: DynamicArrayStrategy) -> Engine {
    let mut engine = Engine::new();
    engine.set_dynamic_array_strategy(strategy);
    engine
}

#[test]
fn dynamic_array_scenarios_pass_for_all_strategies() {
    for strategy in strategies() {
        let mut engine = engine_with_strategy(strategy);

        engine
            .set_formula_a1("A1", "=SEQUENCE(3,2,1,1)")
            .expect("set sequence");
        engine
            .set_formula_a1("C1", "=SUM(A1#)")
            .expect("set spill ref aggregate");
        engine
            .set_formula_a1("D1", "=B2")
            .expect("set interior reference");
        engine
            .set_formula_a1("E1", "=SUM(A1:B3)")
            .expect("set range aggregate");
        engine
            .set_formula_a1("F1", "=A1# + 10")
            .expect("set arrayified formula");

        assert_eq!(
            engine.cell_state_a1("A1").expect("A1").value,
            Value::Number(1.0)
        );
        assert_eq!(
            engine.cell_state_a1("B3").expect("B3").value,
            Value::Number(6.0)
        );
        assert_eq!(
            engine.cell_state_a1("C1").expect("C1").value,
            Value::Number(21.0)
        );
        assert_eq!(
            engine.cell_state_a1("D1").expect("D1").value,
            Value::Number(4.0)
        );
        assert_eq!(
            engine.cell_state_a1("E1").expect("E1").value,
            Value::Number(21.0)
        );
        assert_eq!(
            engine.cell_state_a1("F1").expect("F1").value,
            Value::Number(11.0)
        );
        assert_eq!(
            engine.cell_state_a1("G3").expect("G3").value,
            Value::Number(16.0)
        );

        assert_eq!(
            engine
                .spill_anchor_for_cell_a1("B2")
                .expect("spill anchor for B2"),
            Some(CellRef::from_a1("A1").expect("A1"))
        );
        assert_eq!(
            engine
                .spill_range_for_anchor(CellRef::from_a1("A1").expect("A1"))
                .expect("spill range for A1"),
            Some(CellRange::new(
                CellRef::from_a1("A1").expect("A1"),
                CellRef::from_a1("B3").expect("B3")
            ))
        );

        engine
            .set_formula_a1("A1", "=SEQUENCE(2,2,10,1)")
            .expect("resize sequence");

        assert_eq!(
            engine.cell_state_a1("D1").expect("D1").value,
            Value::Number(13.0)
        );
        assert_eq!(
            engine.cell_state_a1("C1").expect("C1").value,
            Value::Number(46.0)
        );
        assert_eq!(
            engine.cell_state_a1("E1").expect("E1").value,
            Value::Number(46.0)
        );
        assert_eq!(
            engine.cell_state_a1("F1").expect("F1").value,
            Value::Number(20.0)
        );
        assert_eq!(
            engine.cell_state_a1("G2").expect("G2").value,
            Value::Number(23.0)
        );
        assert_eq!(engine.cell_state_a1("G3").expect("G3").value, Value::Blank);

        assert_eq!(
            engine
                .spill_range_for_anchor(CellRef::from_a1("A1").expect("A1"))
                .expect("spill range after resize"),
            Some(CellRange::new(
                CellRef::from_a1("A1").expect("A1"),
                CellRef::from_a1("B2").expect("B2")
            ))
        );
        assert_eq!(
            engine
                .spill_anchor_for_cell_a1("G3")
                .expect("spill anchor for G3 after shrink"),
            None
        );
    }
}

#[test]
fn blocked_spill_reports_error_for_all_strategies() {
    for strategy in strategies() {
        let mut engine = engine_with_strategy(strategy);
        engine.set_number_a1("A2", 99.0).expect("set literal");
        engine
            .set_formula_a1("A1", "=SEQUENCE(3)")
            .expect("set sequence");

        match engine.cell_state_a1("A1").expect("A1").value {
            Value::Error(CellError::Spill(msg)) => assert!(msg.contains("blocked")),
            other => panic!("expected spill error, got {other:?} for strategy {strategy:?}"),
        }

        assert_eq!(
            engine.cell_state_a1("A2").expect("A2").value,
            Value::Number(99.0)
        );
    }
}

#[test]
fn switching_strategies_on_same_engine_keeps_results_consistent() {
    let mut engine = Engine::new();
    engine
        .set_formula_a1("A1", "=SEQUENCE(2,2,1,1)")
        .expect("set sequence");
    engine.set_formula_a1("D1", "=SUM(A1#)").expect("set sum");

    for strategy in strategies() {
        engine.set_dynamic_array_strategy(strategy);
        engine
            .recalculate()
            .expect("recalculate after strategy switch");
        assert_eq!(
            engine.cell_state_a1("D1").expect("D1").value,
            Value::Number(10.0)
        );
        assert_eq!(
            engine.cell_state_a1("B2").expect("B2").value,
            Value::Number(4.0)
        );
    }
}
