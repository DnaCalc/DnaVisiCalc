use dnavisicalc_file::load_from_str;

#[test]
fn duplicate_mode_is_rejected() {
    let input = "DVISICALC\t1\nMODE\tAUTO\nMODE\tMANUAL\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("duplicate MODE"));
}

#[test]
fn unknown_mode_is_rejected() {
    let input = "DVISICALC\t1\nMODE\tMAYBE\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("unknown mode"));
}

#[test]
fn mode_extra_fields_are_rejected() {
    let input = "DVISICALC\t1\nMODE\tAUTO\tEXTRA\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("extra fields"));
}

#[test]
fn missing_cell_fields_are_rejected() {
    let input = "DVISICALC\t1\nCELL\tA1\tN\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("missing value"));
}

#[test]
fn unknown_cell_type_is_rejected() {
    let input = "DVISICALC\t1\nCELL\tA1\tX\t1\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("unknown CELL type"));
}

#[test]
fn malformed_number_is_rejected() {
    let input = "DVISICALC\t1\nCELL\tA1\tN\tnot-a-number\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("invalid number"));
}

#[test]
fn cell_extra_fields_are_rejected() {
    let input = "DVISICALC\t1\nCELL\tA1\tN\t1\textra\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("extra fields"));
}

#[test]
fn missing_name_fields_are_rejected() {
    let input = "DVISICALC\t1\nNAME\tRATE\tN\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("missing value"));
}

#[test]
fn unknown_name_type_is_rejected() {
    let input = "DVISICALC\t1\nNAME\tRATE\tX\t1\n";
    let err = load_from_str(input).expect_err("expected parse error");
    assert!(err.to_string().contains("unknown NAME type"));
}
