use dnavisicalc_core::{
    CellFormat, CellInput, CellRef, ChangeEntry, ChartDefinition, ControlDefinition,
    DiagnosticCode, Engine, EngineError, IterationConfig, NameInput, PaletteColor, RecalcMode,
    UdfHandler, Value, Volatility,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[test]
fn clear_resets_cells_and_values() {
    let mut engine = Engine::new();
    engine.set_number_a1("A1", 5.0).expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");

    engine.clear();

    assert_eq!(engine.all_cell_inputs().len(), 0);
    assert_eq!(
        engine.cell_state_a1("A1").expect("query").value,
        Value::Blank
    );
}

#[test]
fn cell_input_accessors_roundtrip() {
    let mut engine = Engine::new();
    engine
        .set_cell_input_a1("C3", CellInput::Formula("=1+2".to_string()))
        .expect("set C3");
    engine
        .set_cell_input_a1("C4", CellInput::Text("abc".to_string()))
        .expect("set C4");

    let input_c3 = engine.cell_input_a1("C3").expect("query C3");
    assert_eq!(input_c3, Some(CellInput::Formula("=1+2".to_string())));
    let input_c4 = engine.cell_input_a1("C4").expect("query C4");
    assert_eq!(input_c4, Some(CellInput::Text("abc".to_string())));
    assert_eq!(
        engine
            .formula_source_a1("C3")
            .expect("formula source")
            .expect("formula exists"),
        "=1+2"
    );
}

#[test]
fn out_of_bounds_cellref_errors_on_direct_api() {
    let mut engine = Engine::new();
    let bad = CellRef { col: 0, row: 1 };
    let err = engine
        .set_number(bad, 1.0)
        .expect_err("expected bounds error");
    assert!(matches!(err, EngineError::OutOfBounds(_)));
}

#[test]
fn manual_mode_allows_stale_after_clear_cell() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 1.0).expect("set A1");
    engine.set_formula_a1("B1", "=A1+1").expect("set B1");
    engine.recalculate().expect("recalc");

    engine.clear_cell_a1("A1").expect("clear A1");
    let state = engine.cell_state_a1("B1").expect("query B1");
    assert!(state.stale);
}

#[test]
fn name_input_accessors_roundtrip() {
    let mut engine = Engine::new();
    engine
        .set_name_input("tax_rate", NameInput::Number(0.21))
        .expect("set name");
    engine
        .set_name_input(
            "greeting",
            NameInput::Formula("=CONCAT(\"hi\", \" there\")".to_string()),
        )
        .expect("set formula name");

    let name = engine.name_input("TAX_RATE").expect("query TAX_RATE");
    assert_eq!(name, Some(NameInput::Number(0.21)));
    let greeting = engine.name_input("greeting").expect("query greeting");
    assert_eq!(
        greeting,
        Some(NameInput::Formula(
            "=CONCAT(\"hi\", \" there\")".to_string()
        ))
    );
}

#[test]
fn rejects_invalid_name_collisions() {
    let mut engine = Engine::new();
    let err = engine
        .set_name_number("A1", 1.0)
        .expect_err("expected invalid name");
    assert!(err.to_string().contains("conflicts with a cell reference"));

    let err = engine
        .set_name_number("SUM", 1.0)
        .expect_err("expected invalid name");
    assert!(err.to_string().contains("built-in function"));
}

#[test]
fn cell_format_accessors_roundtrip() {
    let mut engine = Engine::new();
    let format = CellFormat {
        decimals: Some(3),
        bold: true,
        italic: true,
        fg: Some(PaletteColor::Sage),
        bg: Some(PaletteColor::Cloud),
    };
    engine
        .set_cell_format_a1("B2", format.clone())
        .expect("set format");

    let loaded = engine.cell_format_a1("B2").expect("get format");
    assert_eq!(loaded, format);
    assert_eq!(engine.all_cell_formats().len(), 1);
}

#[test]
fn formatting_change_does_not_mark_values_stale() {
    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_formula_a1("B1", "=A1*2").expect("B1");
    engine.recalculate().expect("recalc");

    engine
        .set_cell_format_a1(
            "B1",
            CellFormat {
                decimals: Some(1),
                bold: false,
                italic: false,
                fg: None,
                bg: None,
            },
        )
        .expect("set format");

    let state = engine.cell_state_a1("B1").expect("B1");
    assert!(!state.stale);
}

#[test]
fn stream_is_externally_invalidated_not_volatile() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=STREAM(1)").expect("A1");
    assert!(!engine.has_volatile_cells());
    assert!(engine.has_externally_invalidated_cells());

    engine.set_formula_a1("B1", "=NOW()").expect("B1");
    assert!(engine.has_volatile_cells());
}

#[test]
fn invalidate_volatile_marks_rand_dirty_and_recalculates() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=RAND()").expect("A1");
    let before_epoch = engine.committed_epoch();
    let before_value = engine.cell_state_a1("A1").expect("A1").value;

    engine.invalidate_volatile().expect("invalidate volatile");

    let after_epoch = engine.committed_epoch();
    let after_value = engine.cell_state_a1("A1").expect("A1").value;
    assert_eq!(after_epoch, before_epoch + 1);
    assert_ne!(before_value, after_value);
    assert_eq!(engine.last_eval_count(), 1);
}

#[derive(Debug)]
struct ExternalCounterUdf {
    value: Arc<AtomicU64>,
}

impl UdfHandler for ExternalCounterUdf {
    fn call(&self, _args: &[Value]) -> Value {
        Value::Number(self.value.load(Ordering::SeqCst) as f64)
    }

    fn volatility(&self) -> Volatility {
        Volatility::ExternallyInvalidated
    }
}

#[test]
fn invalidate_udf_recalculates_externally_invalidated_calls() {
    let mut engine = Engine::new();
    let shared = Arc::new(AtomicU64::new(0));
    engine.register_udf(
        "EXT_COUNT",
        Box::new(ExternalCounterUdf {
            value: Arc::clone(&shared),
        }),
    );
    engine
        .set_formula_a1("A1", "=EXT_COUNT()")
        .expect("set formula");
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(0.0)
    );

    shared.store(7, Ordering::SeqCst);
    engine.invalidate_udf("EXT_COUNT").expect("invalidate udf");
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(7.0)
    );
}

#[test]
fn control_api_roundtrip_and_name_sync() {
    let mut engine = Engine::new();
    engine
        .define_control("rate", ControlDefinition::slider(0.0, 10.0, 1.0))
        .expect("define control");
    engine
        .set_formula_a1("A1", "=RATE*2")
        .expect("formula using control-backed name");

    engine
        .set_control_value("RATE", 12.0)
        .expect("set control value");
    assert_eq!(engine.control_value("rate"), Some(10.0));
    assert_eq!(
        engine.cell_state_a1("A1").expect("A1").value,
        Value::Number(20.0)
    );

    let controls = engine.all_controls();
    assert_eq!(controls.len(), 1);
    assert_eq!(controls[0].0, "RATE");
    assert_eq!(controls[0].2, 10.0);

    assert!(engine.remove_control("RATE"));
    assert!(engine.control_definition("RATE").is_none());
    assert_eq!(
        engine.name_input("RATE").expect("name input"),
        Some(NameInput::Number(10.0))
    );
}

#[test]
fn chart_output_updates_when_source_cells_change() {
    let mut engine = Engine::new();
    engine.set_number_a1("A2", 1.0).expect("A2");
    engine.set_number_a1("A3", 2.0).expect("A3");
    engine
        .define_chart(
            "sales",
            ChartDefinition {
                source_range: dnavisicalc_core::CellRange::new(
                    CellRef::from_a1("A2").expect("A2"),
                    CellRef::from_a1("A3").expect("A3"),
                ),
            },
        )
        .expect("define chart");

    let initial = engine.chart_output("SALES").expect("chart output");
    assert_eq!(initial.series.len(), 1);
    assert_eq!(initial.series[0].values, vec![1.0, 2.0]);

    engine.set_number_a1("A3", 5.0).expect("A3 update");
    let updated = engine.chart_output("SALES").expect("chart output");
    assert_eq!(updated.series[0].values, vec![1.0, 5.0]);
}

#[test]
fn change_tracking_collects_cell_chart_and_format_changes() {
    let mut engine = Engine::new();
    engine.enable_change_tracking();

    engine.set_number_a1("A1", 42.0).expect("A1");
    engine
        .define_chart(
            "chart_one",
            ChartDefinition {
                source_range: dnavisicalc_core::CellRange::new(
                    CellRef::from_a1("A1").expect("A1"),
                    CellRef::from_a1("A1").expect("A1"),
                ),
            },
        )
        .expect("define chart");
    engine
        .set_cell_format_a1(
            "A1",
            CellFormat {
                decimals: Some(2),
                bold: true,
                italic: false,
                fg: Some(PaletteColor::Sage),
                bg: None,
            },
        )
        .expect("format");

    let entries = engine.drain_changes();
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, ChangeEntry::CellValue { cell, .. } if *cell == CellRef::from_a1("A1").expect("A1"))),
        "expected at least one cell value change for A1",
    );
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, ChangeEntry::ChartOutput { name, .. } if name == "CHART_ONE")),
        "expected chart output change for CHART_ONE",
    );
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, ChangeEntry::CellFormat { cell, .. } if *cell == CellRef::from_a1("A1").expect("A1"))),
        "expected cell format change for A1",
    );
}

#[test]
fn change_tracking_emits_cycle_diagnostic_for_non_iterative_recalc() {
    let mut engine = Engine::new();
    engine.enable_change_tracking();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_formula_a1("A1", "=B1+1").expect("A1");
    engine.set_formula_a1("B1", "=A1+1").expect("B1");
    engine.recalculate().expect("recalc");

    let entries = engine.drain_changes();
    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            ChangeEntry::Diagnostic {
                code: DiagnosticCode::CircularReferenceDetected,
                ..
            }
        )),
        "expected circular-reference diagnostic entry",
    );
}

#[test]
fn iterative_recalc_does_not_emit_non_iterative_cycle_diagnostic() {
    let mut engine = Engine::new();
    engine.enable_change_tracking();
    engine.set_recalc_mode(RecalcMode::Manual);
    engine.set_iteration_config(IterationConfig {
        enabled: true,
        max_iterations: 10,
        convergence_tolerance: 0.001,
    });
    engine.set_formula_a1("A1", "=B1+1").expect("A1");
    engine.set_formula_a1("B1", "=A1+1").expect("B1");
    engine.recalculate().expect("recalc");

    let entries = engine.drain_changes();
    assert!(
        !entries
            .iter()
            .any(|entry| matches!(entry, ChangeEntry::Diagnostic { .. })),
        "did not expect non-iterative cycle diagnostic when iteration is enabled",
    );
}
