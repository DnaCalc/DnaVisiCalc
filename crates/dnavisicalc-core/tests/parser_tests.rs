use dnavisicalc_core::{
    BinaryOp, CellRange, CellRef, DEFAULT_SHEET_BOUNDS, Expr, RefFlags, UnaryOp, parse_formula,
};

#[test]
fn parses_operator_precedence() {
    let expr = parse_formula("=1+2*3", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } => {
            assert_eq!(*left, Expr::Number(1.0));
            match *right {
                Expr::Binary {
                    op: BinaryOp::Mul,
                    left,
                    right,
                } => {
                    assert_eq!(*left, Expr::Number(2.0));
                    assert_eq!(*right, Expr::Number(3.0));
                }
                _ => panic!("expected multiplication on right side"),
            }
        }
        _ => panic!("expected addition at top-level"),
    }
}

#[test]
fn parses_visicalc_style_sum_range() {
    let expr = parse_formula("@SUM(B2...M2)", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::FunctionCall { name, args } => {
            assert_eq!(name, "SUM");
            assert_eq!(args.len(), 1);
            let expected = Expr::Range(
                CellRange::new(
                    CellRef::from_a1("B2").expect("valid cell"),
                    CellRef::from_a1("M2").expect("valid cell"),
                ),
                RefFlags::RELATIVE,
                RefFlags::RELATIVE,
            );
            assert_eq!(args[0], expected);
        }
        _ => panic!("expected function call"),
    }
}

#[test]
fn parses_unary_minus() {
    let expr = parse_formula("=-A1", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Unary {
            op: UnaryOp::Minus,
            expr,
        } => assert_eq!(
            *expr,
            Expr::Cell(
                CellRef::from_a1("A1").expect("valid cell"),
                RefFlags::RELATIVE
            )
        ),
        _ => panic!("expected unary minus"),
    }
}

#[test]
fn rejects_invalid_range_boundary() {
    let err = parse_formula("=SUM(1...A1)", DEFAULT_SHEET_BOUNDS).expect_err("formula should fail");
    assert!(err.message.contains("range boundaries"));
}

#[test]
fn parses_spill_reference_postfix() {
    let expr = parse_formula("=A1#", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    assert_eq!(expr, Expr::SpillRef(CellRef::from_a1("A1").expect("valid")));
}

#[test]
fn parses_string_concat_operator() {
    let expr = parse_formula("=\"hi\"&A1", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Binary {
            op: BinaryOp::Concat,
            left,
            right,
        } => {
            assert_eq!(*left, Expr::Text("hi".to_string()));
            assert_eq!(
                *right,
                Expr::Cell(CellRef::from_a1("A1").expect("valid"), RefFlags::RELATIVE)
            );
        }
        _ => panic!("expected concat expression"),
    }
}

#[test]
fn parses_escaped_quote_in_string_literal() {
    let expr = parse_formula("=\"a\"\"b\"", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    assert_eq!(expr, Expr::Text("a\"b".to_string()));
}

#[test]
fn parses_function_name_that_looks_like_cell_ref() {
    let expr = parse_formula("=LOG10(100)", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::FunctionCall { name, args } => {
            assert_eq!(name, "LOG10");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected function call"),
    }
}

#[test]
fn parses_named_reference() {
    let expr = parse_formula("=tax_rate*B2", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Binary {
            op: BinaryOp::Mul,
            left,
            right,
        } => {
            assert_eq!(*left, Expr::Name("TAX_RATE".to_string()));
            assert_eq!(
                *right,
                Expr::Cell(CellRef::from_a1("B2").expect("valid"), RefFlags::RELATIVE)
            );
        }
        _ => panic!("expected multiply expression"),
    }
}

#[test]
fn parses_name_starting_with_underscore() {
    let expr = parse_formula("=_discount+1", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } => {
            assert_eq!(*left, Expr::Name("_DISCOUNT".to_string()));
            assert_eq!(*right, Expr::Number(1.0));
        }
        _ => panic!("expected addition"),
    }
}

#[test]
fn parses_let_and_lambda_calls() {
    let expr = parse_formula("=LET(x,10,f,LAMBDA(v,v+1),f(x))", DEFAULT_SHEET_BOUNDS)
        .expect("formula should parse");
    match expr {
        Expr::FunctionCall { name, args } => {
            assert_eq!(name, "LET");
            assert_eq!(args.len(), 5);
        }
        _ => panic!("expected LET function call"),
    }
}

#[test]
fn parses_indirect_offset_row_column() {
    let expr = parse_formula(
        "=SUM(INDIRECT(\"A1\"),OFFSET(A1,1,1),ROW(B2),COLUMN(C3))",
        DEFAULT_SHEET_BOUNDS,
    )
    .expect("formula should parse");
    match expr {
        Expr::FunctionCall { name, args } => {
            assert_eq!(name, "SUM");
            assert_eq!(args.len(), 4);
        }
        _ => panic!("expected function call"),
    }
}

#[test]
fn parses_direct_lambda_invoke() {
    let expr =
        parse_formula("=(LAMBDA(x,x+1))(5)", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    match expr {
        Expr::Invoke { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(*callee, Expr::FunctionCall { .. }));
        }
        _ => panic!("expected invoke expression"),
    }
}
