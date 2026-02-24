use dnavisicalc_core::Value;
use dnavisicalc_tui::{Action, App, CommandOutcome, MemoryWorkbookIo};

fn run_command(app: &mut App, io: &mut MemoryWorkbookIo, command: &str) -> CommandOutcome {
    app.apply(Action::StartCommand, io);
    for ch in command.chars() {
        app.apply(Action::InputChar(ch), io);
    }
    app.apply(Action::Submit, io)
}

#[test]
fn opening_broken_file_surfaces_error_and_keeps_app_alive() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();
    io.insert_file("bad.dvc", "DVISICALC\t1\nCELL\tA1\tF\t=\n");

    let outcome = run_command(&mut app, &mut io, "o bad.dvc");
    assert_eq!(outcome, CommandOutcome::Continue);
    assert!(app.status().contains("Open error"));

    app.apply(Action::StartEdit, &mut io);
    app.apply(Action::InputChar('9'), &mut io);
    app.apply(Action::Submit, &mut io);

    assert_eq!(
        app.engine().cell_state_a1("A1").expect("query A1").value,
        Value::Number(9.0)
    );
}

#[test]
fn open_missing_file_is_non_fatal() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let outcome = run_command(&mut app, &mut io, "o missing.dvc");
    assert_eq!(outcome, CommandOutcome::Continue);
    assert!(app.status().contains("Open error"));
}
