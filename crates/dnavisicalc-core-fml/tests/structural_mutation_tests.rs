use dnavisicalc_core::{
    CellRef, DEFAULT_SHEET_BOUNDS, Engine, EngineError, Expr, RefFlags, StructuralOp, Value,
    expr_to_formula, rewrite_expr,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn cell(col: u16, row: u16) -> CellRef {
    CellRef::new(col, row, DEFAULT_SHEET_BOUNDS).unwrap()
}

fn engine_with(cells: &[(&str, &str)]) -> Engine {
    let mut engine = Engine::new();
    for (addr, value) in cells {
        if value.starts_with('=') || value.starts_with('@') {
            engine.set_formula_a1(addr, value).expect("set formula");
        } else if let Ok(n) = value.parse::<f64>() {
            engine.set_number_a1(addr, n).expect("set number");
        } else {
            engine.set_text_a1(addr, *value).expect("set text");
        }
    }
    engine
}

fn val(engine: &Engine, addr: &str) -> Value {
    engine.cell_state_a1(addr).expect("query").value
}

// ---------------------------------------------------------------------------
// Parser tests for absolute references ($)
// ---------------------------------------------------------------------------

#[test]
fn parses_absolute_column_reference() {
    let expr = dnavisicalc_core::parse_formula("=$A1", DEFAULT_SHEET_BOUNDS)
        .expect("formula should parse");
    let expected_flags = RefFlags {
        col_absolute: true,
        row_absolute: false,
    };
    assert_eq!(
        expr,
        Expr::Cell(CellRef::from_a1("A1").unwrap(), expected_flags)
    );
}

#[test]
fn parses_absolute_row_reference() {
    let expr = dnavisicalc_core::parse_formula("=A$1", DEFAULT_SHEET_BOUNDS)
        .expect("formula should parse");
    let expected_flags = RefFlags {
        col_absolute: false,
        row_absolute: true,
    };
    assert_eq!(
        expr,
        Expr::Cell(CellRef::from_a1("A1").unwrap(), expected_flags)
    );
}

#[test]
fn parses_fully_absolute_reference() {
    let expr = dnavisicalc_core::parse_formula("=$A$1", DEFAULT_SHEET_BOUNDS)
        .expect("formula should parse");
    assert_eq!(
        expr,
        Expr::Cell(CellRef::from_a1("A1").unwrap(), RefFlags::ABSOLUTE)
    );
}

#[test]
fn parses_relative_reference_unchanged() {
    let expr =
        dnavisicalc_core::parse_formula("=A1", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    assert_eq!(
        expr,
        Expr::Cell(CellRef::from_a1("A1").unwrap(), RefFlags::RELATIVE)
    );
}

// ---------------------------------------------------------------------------
// expr_to_formula round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn expr_to_formula_roundtrip_simple_addition() {
    let expr = Expr::Binary {
        op: dnavisicalc_core::BinaryOp::Add,
        left: Box::new(Expr::Cell(cell(1, 1), RefFlags::RELATIVE)),
        right: Box::new(Expr::Number(1.0)),
    };
    let formula = expr_to_formula(&expr);
    assert_eq!(formula, "=(A1+1)");
}

#[test]
fn expr_to_formula_preserves_absolute_flags() {
    let expr = Expr::Cell(cell(2, 3), RefFlags::ABSOLUTE);
    let formula = expr_to_formula(&expr);
    assert_eq!(formula, "=$B$3");
}

#[test]
fn expr_to_formula_mixed_absolute() {
    let expr = Expr::Cell(
        cell(2, 3),
        RefFlags {
            col_absolute: true,
            row_absolute: false,
        },
    );
    let formula = expr_to_formula(&expr);
    assert_eq!(formula, "=$B3");
}

// ---------------------------------------------------------------------------
// rewrite_expr tests
// ---------------------------------------------------------------------------

#[test]
fn rewrite_shifts_relative_ref_on_insert_row() {
    // =A5 with insert at row 3 → =A6
    let expr = Expr::Cell(cell(1, 5), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::InsertRow { at: 3 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, Some(Expr::Cell(cell(1, 6), RefFlags::RELATIVE)));
}

#[test]
fn rewrite_does_not_shift_ref_before_insert_point() {
    // =A2 with insert at row 5 → =A2 (unchanged)
    let expr = Expr::Cell(cell(1, 2), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::InsertRow { at: 5 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, Some(Expr::Cell(cell(1, 2), RefFlags::RELATIVE)));
}

#[test]
fn rewrite_shifts_absolute_ref_on_insert_row() {
    // =$A$5 with insert at row 3 → =$A$6
    let expr = Expr::Cell(cell(1, 5), RefFlags::ABSOLUTE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::InsertRow { at: 3 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, Some(Expr::Cell(cell(1, 6), RefFlags::ABSOLUTE)));
}

#[test]
fn rewrite_invalidates_ref_on_delete_row() {
    // =A5 and we delete row 5 → None (#REF!)
    let expr = Expr::Cell(cell(1, 5), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::DeleteRow { at: 5 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, None);
}

#[test]
fn rewrite_shifts_ref_after_deleted_row() {
    // =A5 and we delete row 3 → =A4
    let expr = Expr::Cell(cell(1, 5), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::DeleteRow { at: 3 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, Some(Expr::Cell(cell(1, 4), RefFlags::RELATIVE)));
}

#[test]
fn rewrite_shifts_column_on_insert_col() {
    // =C1 with insert at col 2 (B) → =D1
    let expr = Expr::Cell(cell(3, 1), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::InsertCol { at: 2 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, Some(Expr::Cell(cell(4, 1), RefFlags::RELATIVE)));
}

#[test]
fn rewrite_invalidates_ref_on_delete_col() {
    // =B1 and we delete col 2 (B) → None
    let expr = Expr::Cell(cell(2, 1), RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::DeleteCol { at: 2 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, None);
}

#[test]
fn rewrite_handles_range_insert_row() {
    // =SUM(A1:A10) with insert at row 5 → =SUM(A1:A11)
    let range = dnavisicalc_core::CellRange::new(cell(1, 1), cell(1, 10));
    let expr = Expr::Range(range, RefFlags::RELATIVE, RefFlags::RELATIVE);
    let result = rewrite_expr(
        &expr,
        StructuralOp::InsertRow { at: 5 },
        DEFAULT_SHEET_BOUNDS,
    );
    let expected_range = dnavisicalc_core::CellRange::new(cell(1, 1), cell(1, 11));
    assert_eq!(
        result,
        Some(Expr::Range(
            expected_range,
            RefFlags::RELATIVE,
            RefFlags::RELATIVE
        ))
    );
}

#[test]
fn rewrite_propagates_invalidation_through_function_call() {
    // =SUM(A5) where A5 is in the deleted row → None
    let expr = Expr::FunctionCall {
        name: "SUM".to_string(),
        args: vec![Expr::Cell(cell(1, 5), RefFlags::RELATIVE)],
    };
    let result = rewrite_expr(
        &expr,
        StructuralOp::DeleteRow { at: 5 },
        DEFAULT_SHEET_BOUNDS,
    );
    assert_eq!(result, None);
}

// ---------------------------------------------------------------------------
// Engine insert_row tests
// ---------------------------------------------------------------------------

#[test]
fn insert_row_shifts_cells_down() {
    let mut engine = engine_with(&[("A1", "10"), ("A2", "20"), ("A3", "30")]);
    engine.insert_row(2).expect("insert");

    assert_eq!(val(&engine, "A1"), Value::Number(10.0));
    assert_eq!(val(&engine, "A2"), Value::Blank); // new empty row
    assert_eq!(val(&engine, "A3"), Value::Number(20.0)); // was A2
    assert_eq!(val(&engine, "A4"), Value::Number(30.0)); // was A3
}

#[test]
fn insert_row_rewrites_formula_references() {
    let mut engine = engine_with(&[
        ("A1", "10"),
        ("A2", "20"),
        ("B1", "=A1+A2"), // references A1 and A2
    ]);

    engine.insert_row(2).expect("insert");

    // A2 was shifted to A3, so B1's formula should now be =A1+A3
    // B1 itself is at row 1 — not affected by insert at row 2
    let b1_val = val(&engine, "B1");
    assert_eq!(b1_val, Value::Number(30.0)); // 10 + 20

    // Check that the formula source was rewritten
    let source = engine
        .formula_source_a1("B1")
        .expect("query")
        .expect("has formula");
    assert!(
        source.contains("A3"),
        "formula should reference A3, got: {source}"
    );
}

#[test]
fn insert_row_shifts_formula_cell_position() {
    let mut engine = engine_with(&[
        ("A1", "10"),
        ("A3", "=A1*2"), // formula at row 3
    ]);

    engine.insert_row(2).expect("insert");

    // Formula was at A3, should now be at A4
    assert_eq!(val(&engine, "A3"), Value::Blank);
    assert_eq!(val(&engine, "A4"), Value::Number(20.0));
}

// ---------------------------------------------------------------------------
// Engine delete_row tests
// ---------------------------------------------------------------------------

#[test]
fn delete_row_shifts_cells_up() {
    let mut engine = engine_with(&[("A1", "10"), ("A2", "20"), ("A3", "30")]);
    engine.delete_row(2).expect("delete");

    assert_eq!(val(&engine, "A1"), Value::Number(10.0));
    assert_eq!(val(&engine, "A2"), Value::Number(30.0)); // was A3
    assert_eq!(val(&engine, "A3"), Value::Blank);
}

#[test]
fn delete_row_invalidates_formula_referencing_deleted_row() {
    let mut engine = engine_with(&[
        ("A1", "10"),
        ("A2", "20"),
        ("B1", "=A2"), // references the row we'll delete
    ]);

    engine.delete_row(2).expect("delete");

    // B1's reference to A2 should be invalidated — becomes #REF! text
    assert_eq!(val(&engine, "B1"), Value::Text("#REF!".to_string()));
}

#[test]
fn delete_row_adjusts_references_after_deleted_row() {
    let mut engine = engine_with(&[
        ("A1", "10"),
        ("A2", "20"),
        ("A3", "30"),
        ("B1", "=A3"), // references row 3
    ]);

    engine.delete_row(2).expect("delete");

    // A3 shifted to A2, B1's formula should reference A2
    assert_eq!(val(&engine, "B1"), Value::Number(30.0));
    let source = engine
        .formula_source_a1("B1")
        .expect("query")
        .expect("has formula");
    assert!(
        source.contains("A2"),
        "formula should reference A2, got: {source}"
    );
}

// ---------------------------------------------------------------------------
// Engine insert_col tests
// ---------------------------------------------------------------------------

#[test]
fn insert_col_shifts_cells_right() {
    let mut engine = engine_with(&[("A1", "10"), ("B1", "20"), ("C1", "30")]);
    engine.insert_col(2).expect("insert");

    assert_eq!(val(&engine, "A1"), Value::Number(10.0));
    assert_eq!(val(&engine, "B1"), Value::Blank); // new empty column
    assert_eq!(val(&engine, "C1"), Value::Number(20.0)); // was B1
    assert_eq!(val(&engine, "D1"), Value::Number(30.0)); // was C1
}

#[test]
fn insert_col_rewrites_formula_references() {
    let mut engine = engine_with(&[("A1", "10"), ("B1", "20"), ("A2", "=A1+B1")]);

    engine.insert_col(2).expect("insert"); // insert at column B

    // B1 shifted to C1, formula should now be =A1+C1
    assert_eq!(val(&engine, "A2"), Value::Number(30.0));
    let source = engine
        .formula_source_a1("A2")
        .expect("query")
        .expect("has formula");
    assert!(
        source.contains("C1"),
        "formula should reference C1, got: {source}"
    );
}

// ---------------------------------------------------------------------------
// Engine delete_col tests
// ---------------------------------------------------------------------------

#[test]
fn delete_col_shifts_cells_left() {
    let mut engine = engine_with(&[("A1", "10"), ("B1", "20"), ("C1", "30")]);
    engine.delete_col(2).expect("delete");

    assert_eq!(val(&engine, "A1"), Value::Number(10.0));
    assert_eq!(val(&engine, "B1"), Value::Number(30.0)); // was C1
    assert_eq!(val(&engine, "C1"), Value::Blank);
}

#[test]
fn delete_col_invalidates_formula_referencing_deleted_col() {
    let mut engine = engine_with(&[("A1", "10"), ("B1", "20"), ("A2", "=B1")]);

    engine.delete_col(2).expect("delete");

    // A2's reference to B1 should be invalidated — becomes #REF! text
    assert_eq!(val(&engine, "A2"), Value::Text("#REF!".to_string()));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn insert_row_at_boundary_rejects_out_of_bounds() {
    let mut engine = Engine::new();
    assert!(matches!(
        engine.insert_row(0).expect_err("insert_row(0)"),
        EngineError::OutOfBounds(CellRef { col: 1, row: 0 })
    ));
    assert!(matches!(
        engine.insert_row(255).expect_err("insert_row(255)"),
        EngineError::OutOfBounds(CellRef { col: 1, row: 255 })
    ));
}

#[test]
fn delete_row_at_boundary_rejects_out_of_bounds() {
    let mut engine = Engine::new();
    assert!(matches!(
        engine.delete_row(0).expect_err("delete_row(0)"),
        EngineError::OutOfBounds(CellRef { col: 1, row: 0 })
    ));
    assert!(matches!(
        engine.delete_row(255).expect_err("delete_row(255)"),
        EngineError::OutOfBounds(CellRef { col: 1, row: 255 })
    ));
}

#[test]
fn insert_col_at_boundary_rejects_out_of_bounds() {
    let mut engine = Engine::new();
    assert!(matches!(
        engine.insert_col(0).expect_err("insert_col(0)"),
        EngineError::OutOfBounds(CellRef { col: 0, row: 1 })
    ));
    assert!(matches!(
        engine.insert_col(64).expect_err("insert_col(64)"),
        EngineError::OutOfBounds(CellRef { col: 64, row: 1 })
    ));
}

#[test]
fn delete_col_at_boundary_rejects_out_of_bounds() {
    let mut engine = Engine::new();
    assert!(matches!(
        engine.delete_col(0).expect_err("delete_col(0)"),
        EngineError::OutOfBounds(CellRef { col: 0, row: 1 })
    ));
    assert!(matches!(
        engine.delete_col(64).expect_err("delete_col(64)"),
        EngineError::OutOfBounds(CellRef { col: 64, row: 1 })
    ));
}

#[test]
fn insert_row_preserves_cell_formatting() {
    let mut engine = Engine::new();
    engine.set_number_a1("A2", 42.0).expect("set");
    engine
        .set_cell_format_a1(
            "A2",
            dnavisicalc_core::CellFormat {
                bold: true,
                ..Default::default()
            },
        )
        .expect("fmt");

    engine.insert_row(1).expect("insert");

    // A2 was shifted to A3
    let fmt = engine.cell_format_a1("A3").expect("fmt");
    assert!(fmt.bold, "formatting should have shifted with the cell");
    let fmt_old = engine.cell_format_a1("A2").expect("fmt");
    assert!(!fmt_old.bold, "old position should have default format");
}

#[test]
fn insert_row_with_range_formula_expands_range() {
    let mut engine = engine_with(&[("A1", "1"), ("A2", "2"), ("A3", "3"), ("B1", "=SUM(A1:A3)")]);

    assert_eq!(val(&engine, "B1"), Value::Number(6.0));

    engine.insert_row(2).expect("insert");

    // Range A1:A3 should become A1:A4 (end shifted because row 3 → row 4)
    // New A2 is blank, old A2 (value 2) is now A3, old A3 (value 3) is now A4
    // SUM(A1:A4) = 1 + 0 + 2 + 3 = 6
    assert_eq!(val(&engine, "B1"), Value::Number(6.0));

    let source = engine
        .formula_source_a1("B1")
        .expect("query")
        .expect("formula");
    assert!(
        source.contains("A4"),
        "range end should shift to A4, got: {source}"
    );
}

#[test]
fn delete_row_with_name_formula_rewrites_name() {
    let mut engine = Engine::new();
    engine.set_number_a1("A3", 100.0).expect("set");
    engine.set_name_formula("total", "=A3").expect("set name");

    engine.delete_row(1).expect("delete");

    // A3 shifted to A2. The name formula should reference A2 now.
    // We can verify by checking the computed name value through a cell that references it.
    engine.set_formula_a1("B1", "=total").expect("set");
    assert_eq!(val(&engine, "B1"), Value::Number(100.0));
}

#[test]
fn multiple_structural_ops_in_sequence() {
    let mut engine = engine_with(&[
        ("A1", "1"),
        ("A2", "2"),
        ("A3", "3"),
        ("A4", "4"),
        ("B1", "=A1+A4"), // 1 + 4 = 5
    ]);

    assert_eq!(val(&engine, "B1"), Value::Number(5.0));

    // Insert row at 2: A2→A3, A3→A4, A4→A5
    engine.insert_row(2).expect("insert");
    // B1 formula should now reference A1 and A5
    assert_eq!(val(&engine, "B1"), Value::Number(5.0));

    // Delete row at 3 (which was old A2=2, now at A3)
    engine.delete_row(3).expect("delete");
    // A4→A3, A5→A4. B1 formula should reference A1 and A4
    assert_eq!(val(&engine, "B1"), Value::Number(5.0));
}
