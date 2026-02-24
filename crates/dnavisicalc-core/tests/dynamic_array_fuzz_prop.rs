use dnavisicalc_core::{CellError, DynamicArrayStrategy, Engine, Value, col_index_to_label};
use proptest::prelude::*;

fn cell_label(col: u16, row: u16) -> String {
    format!("{}{}", col_index_to_label(col), row)
}

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

proptest! {
    #[test]
    fn interior_spill_cell_references_match_sequence_values(
        rows in 1u16..8,
        cols in 1u16..8,
        start in -100.0f64..100.0,
        step in -10.0f64..10.0,
        row_offset in 0u16..8,
        col_offset in 0u16..8,
    ) {
        prop_assume!(row_offset < rows);
        prop_assume!(col_offset < cols);

        for strategy in strategies() {
            let mut engine = engine_with_strategy(strategy);
            let formula = format!("=SEQUENCE({rows},{cols},{start},{step})");
            engine.set_formula_a1("A1", &formula).expect("set sequence");

            let src_row = 1 + row_offset;
            let src_col = 1 + col_offset;
            let src_label = cell_label(src_col, src_row);
            engine
                .set_formula_a1("M1", &format!("={src_label}"))
                .expect("set reference formula");

            let n = row_offset as f64 * cols as f64 + col_offset as f64;
            let expected = start + step * n;

            prop_assert_eq!(
                engine.cell_state_a1("M1").expect("M1 state").value,
                Value::Number(expected),
                "strategy={:?}",
                strategy
            );
        }
    }

    #[test]
    fn sum_over_spill_ref_matches_sequence_arithmetic_sum(
        rows in 1u16..8,
        cols in 1u16..8,
        start in -100.0f64..100.0,
        step in -10.0f64..10.0,
    ) {
        prop_assume!(rows > 1 || cols > 1);
        for strategy in strategies() {
            let mut engine = engine_with_strategy(strategy);
            engine
                .set_formula_a1("A1", &format!("=SEQUENCE({rows},{cols},{start},{step})"))
                .expect("set sequence");
            engine.set_formula_a1("M1", "=SUM(A1#)").expect("set sum");

            let count = (rows as f64) * (cols as f64);
            let expected = count * (2.0 * start + (count - 1.0) * step) / 2.0;
            match engine.cell_state_a1("M1").expect("M1 state").value {
                Value::Number(n) => prop_assert!((n - expected).abs() <= 1e-8, "strategy={strategy:?}"),
                other => prop_assert!(false, "expected numeric sum, got {other:?}, strategy={strategy:?}"),
            }
        }
    }

    #[test]
    fn blocked_spill_never_overwrites_existing_input(
        rows in 2u16..8,
        cols in 1u16..8,
        block_row_offset in 1u16..8,
        block_col_offset in 0u16..8,
        blocker in -100.0f64..100.0,
    ) {
        prop_assume!(block_row_offset < rows);
        prop_assume!(block_col_offset < cols);

        for strategy in strategies() {
            let mut engine = engine_with_strategy(strategy);
            let block_row = 1 + block_row_offset;
            let block_col = 1 + block_col_offset;
            let block_label = cell_label(block_col, block_row);
            engine
                .set_number_a1(&block_label, blocker)
                .expect("set blocker");
            engine
                .set_formula_a1("A1", &format!("=SEQUENCE({rows},{cols},1,1)"))
                .expect("set sequence");

            match engine.cell_state_a1("A1").expect("A1 state").value {
                Value::Error(CellError::Spill(_)) => {}
                other => prop_assert!(false, "expected spill error, got {other:?}, strategy={strategy:?}"),
            }
            prop_assert_eq!(
                engine.cell_state_a1(&block_label).expect("block state").value,
                Value::Number(blocker),
                "strategy={:?}",
                strategy
            );
        }
    }

    #[test]
    fn randarray_values_stay_within_bounds(
        rows in 1u16..6,
        cols in 1u16..6,
        min in -10i32..11,
        span in 0i32..20,
        whole in any::<bool>(),
    ) {
        let min = min as f64;
        let max = (min as i32 + span + 1) as f64;
        for strategy in strategies() {
            let mut engine = engine_with_strategy(strategy);
            engine
                .set_formula_a1("A1", &format!("=RANDARRAY({rows},{cols},{min},{max},{whole})"))
                .expect("set randarray");

            for r in 0..rows {
                for c in 0..cols {
                    let label = cell_label(1 + c, 1 + r);
                    match engine.cell_state_a1(&label).expect("rand cell").value {
                        Value::Number(n) => {
                            prop_assert!(n >= min && n <= max, "value {n} out of [{min}, {max}], strategy={strategy:?}");
                            if whole {
                                prop_assert_eq!(n.fract(), 0.0, "strategy={:?}", strategy);
                            }
                        }
                        other => prop_assert!(false, "expected numeric randarray cell, got {other:?}, strategy={strategy:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn deterministic_spill_results_match_across_strategies(
        rows in 2u16..8,
        cols in 2u16..8,
        start in -100.0f64..100.0,
        step in -10.0f64..10.0,
    ) {
        let mut engines = Vec::new();
        for strategy in strategies() {
            let mut engine = engine_with_strategy(strategy);
            engine
                .set_formula_a1("A1", &format!("=SEQUENCE({rows},{cols},{start},{step})"))
                .expect("set sequence");
            engine
                .set_formula_a1("M1", "=SUM(A1#)")
                .expect("set spill ref aggregate");
            engine
                .set_formula_a1("M2", "=B2")
                .expect("set interior direct reference");
            engine
                .set_formula_a1("M3", "=A1# + 5")
                .expect("set arrayified formula");
            engines.push((strategy, engine));
        }

        let baseline = &engines[0].1;
        for (strategy, engine) in &engines[1..] {
            prop_assert_eq!(
                engine
                    .spill_range_for_cell_a1("A1")
                    .expect("spill range for A1"),
                baseline
                    .spill_range_for_cell_a1("A1")
                    .expect("baseline spill range for A1"),
                "strategy={:?}",
                strategy
            );

            for row in 1u16..=8u16 {
                for col in 1u16..=16u16 {
                    let label = cell_label(col, row);
                    prop_assert_eq!(
                        engine.cell_state_a1(&label).expect("cell state").value,
                        baseline.cell_state_a1(&label).expect("baseline cell state").value,
                        "cell={}, strategy={:?}",
                        label,
                        strategy
                    );
                }
            }
        }
    }
}
