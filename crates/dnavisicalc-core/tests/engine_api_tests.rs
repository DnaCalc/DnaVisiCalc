use dnavisicalc_core::{
    CellFormat, CellInput, CellRef, Engine, EngineError, NameInput, PaletteColor, RecalcMode, Value,
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
