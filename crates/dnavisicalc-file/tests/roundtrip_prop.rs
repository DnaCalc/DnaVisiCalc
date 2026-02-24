use dnavisicalc_core::{CellInput, Engine, RecalcMode};
use dnavisicalc_file::{load_from_str, save_to_string};
use proptest::prelude::*;

fn cell_entry_strategy() -> impl Strategy<Value = (u16, u16, bool, f64)> {
    (
        1u16..=63u16,
        1u16..=254u16,
        any::<bool>(),
        (-1000f64..1000f64).prop_filter("finite", |v| v.is_finite()),
    )
}

proptest! {
    #[test]
    fn random_workbooks_roundtrip(entries in prop::collection::vec(cell_entry_strategy(), 0..40), manual in any::<bool>()) {
        let mut engine = Engine::new();
        engine.set_recalc_mode(RecalcMode::Manual);

        for (col, row, as_formula, value) in entries {
            let addr = format!("{}{}", dnavisicalc_core::col_index_to_label(col), row);
            if as_formula {
                let formula = format!("={}", value.round());
                let _ = engine.set_cell_input_a1(&addr, CellInput::Formula(formula));
            } else {
                let _ = engine.set_cell_input_a1(&addr, CellInput::Number(value));
            }
        }

        if manual {
            engine.set_recalc_mode(RecalcMode::Manual);
        } else {
            engine.set_recalc_mode(RecalcMode::Automatic);
        }

        let serialized = save_to_string(&engine).expect("serialize workbook");
        let loaded = load_from_str(&serialized).expect("load workbook");

        prop_assert_eq!(loaded.recalc_mode(), engine.recalc_mode());
        prop_assert_eq!(loaded.all_cell_inputs(), engine.all_cell_inputs());
    }

    #[test]
    fn random_text_input_never_panics(raw in "[ -~]{0,300}") {
        let _ = load_from_str(&raw);
    }
}