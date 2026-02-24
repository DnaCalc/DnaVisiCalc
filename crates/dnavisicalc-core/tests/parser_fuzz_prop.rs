use dnavisicalc_core::{Engine, DEFAULT_SHEET_BOUNDS, parse_formula};
use proptest::prelude::*;

fn formulaish_string() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            'A', 'B', 'C', 'X', 'Y', 'Z',
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
            '+', '-', '*', '/', '^', '=', '<', '>', '(', ')', ':', '.', ',', '@', ' ',
        ]),
        0..96,
    )
    .prop_map(|chars| chars.into_iter().collect::<String>())
}

proptest! {
    #[test]
    fn parser_never_panics_on_formulaish_input(raw in formulaish_string()) {
        let _ = parse_formula(&raw, DEFAULT_SHEET_BOUNDS);
    }

    #[test]
    fn engine_formula_setter_never_panics(raw in formulaish_string()) {
        let mut engine = Engine::new();
        let formula = if raw.starts_with('=') { raw } else { format!("={raw}") };
        let _ = engine.set_formula_a1("A1", &formula);
    }
}

#[test]
fn parser_handles_large_nested_parens_without_panic() {
    let mut formula = String::from("=");
    for _ in 0..120 {
        formula.push('(');
    }
    formula.push_str("1");
    for _ in 0..120 {
        formula.push(')');
    }

    let _ = parse_formula(&formula, DEFAULT_SHEET_BOUNDS);
}