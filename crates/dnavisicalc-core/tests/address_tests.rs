use dnavisicalc_core::address::parse_cell_ref;
use dnavisicalc_core::{
    AddressError, CellRef, DEFAULT_SHEET_BOUNDS, col_index_to_label, col_label_to_index,
};

#[test]
fn column_label_roundtrip_edges() {
    assert_eq!(col_index_to_label(1), "A");
    assert_eq!(col_index_to_label(63), "BK");
    assert_eq!(col_index_to_label(0), "");
    assert_eq!(col_label_to_index("A").expect("A"), 1);
    assert_eq!(col_label_to_index("BK").expect("BK"), 63);
}

#[test]
fn rejects_invalid_column_labels() {
    let err = col_label_to_index("A1").expect_err("invalid");
    match err {
        AddressError::InvalidColumnLabel(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn parse_cell_ref_rejects_invalid_formats() {
    assert!(parse_cell_ref("", DEFAULT_SHEET_BOUNDS).is_err());
    assert!(parse_cell_ref("123", DEFAULT_SHEET_BOUNDS).is_err());
    assert!(parse_cell_ref("A", DEFAULT_SHEET_BOUNDS).is_err());
    assert!(parse_cell_ref("A0", DEFAULT_SHEET_BOUNDS).is_err());
    assert!(parse_cell_ref("BL1", DEFAULT_SHEET_BOUNDS).is_err());
}

#[test]
fn cell_ref_from_str_works() {
    let cell = "B12".parse::<CellRef>().expect("parse B12");
    assert_eq!(cell.col, 2);
    assert_eq!(cell.row, 12);
}
