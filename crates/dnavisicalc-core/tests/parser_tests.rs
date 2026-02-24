use dnavisicalc_core::{
    BinaryOp, CellRange, CellRef, DEFAULT_SHEET_BOUNDS, Expr, UnaryOp, parse_formula,
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
            let expected = Expr::Range(CellRange::new(
                CellRef::from_a1("B2").expect("valid cell"),
                CellRef::from_a1("M2").expect("valid cell"),
            ));
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
            Expr::Cell(CellRef::from_a1("A1").expect("valid cell"))
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
            assert_eq!(*right, Expr::Cell(CellRef::from_a1("A1").expect("valid")));
        }
        _ => panic!("expected concat expression"),
    }
}

#[test]
fn parses_escaped_quote_in_string_literal() {
    let expr = parse_formula("=\"a\"\"b\"", DEFAULT_SHEET_BOUNDS).expect("formula should parse");
    assert_eq!(expr, Expr::Text("a\"b".to_string()));
}
