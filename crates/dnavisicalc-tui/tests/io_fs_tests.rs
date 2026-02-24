use tempfile::tempdir;

use dnavisicalc_core::{Engine, Value};
use dnavisicalc_tui::{FsWorkbookIo, WorkbookIo};

#[test]
fn fs_io_can_save_and_load_workbook() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("sheet.dvc");

    let mut engine = Engine::new();
    engine.set_number_a1("A1", 7.0).expect("set A1");

    let mut io = FsWorkbookIo;
    io.save(path.to_str().expect("utf8 path"), &engine)
        .expect("save workbook");

    let loaded = io
        .load(path.to_str().expect("utf8 path"))
        .expect("load workbook");
    assert_eq!(
        loaded.cell_state_a1("A1").expect("query A1").value,
        Value::Number(7.0)
    );
}
