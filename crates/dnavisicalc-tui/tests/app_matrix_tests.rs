use dnavisicalc_engine::Value;
use dnavisicalc_tui::{Action, App, AppMode, CommandOutcome, MemoryWorkbookIo};

fn run_command(app: &mut App, io: &mut MemoryWorkbookIo, command: &str) -> CommandOutcome {
    app.apply(Action::StartCommand, io);
    for ch in command.chars() {
        app.apply(Action::InputChar(ch), io);
    }
    app.apply(Action::Submit, io)
}

#[test]
fn navigation_clamps_to_sheet_bounds() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    for _ in 0..10 {
        app.apply(Action::MoveLeft, &mut io);
        app.apply(Action::MoveUp, &mut io);
    }

    assert_eq!(app.selected_cell().to_string(), "A1");
}

#[test]
fn edit_mode_accepts_text_cells() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    app.apply(Action::StartEdit, &mut io);
    app.apply(Action::InputChar('h'), &mut io);
    app.apply(Action::InputChar('i'), &mut io);
    app.apply(Action::Submit, &mut io);

    assert_eq!(app.mode(), AppMode::Navigate);
    assert!(app.status().contains("Set A1"));
    assert_eq!(
        app.engine().cell_state_a1("A1").expect("query A1").value,
        Value::Text("hi".to_string())
    );
}

#[test]
fn write_then_open_restores_saved_state() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    app.apply(Action::StartEdit, &mut io);
    app.apply(Action::InputChar('1'), &mut io);
    app.apply(Action::Submit, &mut io);

    let outcome = run_command(&mut app, &mut io, "w sheet1.dvc");
    assert_eq!(outcome, CommandOutcome::Continue);

    app.apply(Action::StartEdit, &mut io);
    app.apply(Action::Backspace, &mut io);
    app.apply(Action::InputChar('2'), &mut io);
    app.apply(Action::Submit, &mut io);
    assert_eq!(
        app.engine().cell_state_a1("A1").expect("query").value,
        Value::Number(2.0)
    );

    let outcome = run_command(&mut app, &mut io, "o sheet1.dvc");
    assert_eq!(outcome, CommandOutcome::Continue);
    assert_eq!(
        app.engine().cell_state_a1("A1").expect("query").value,
        Value::Number(1.0)
    );
}

#[test]
fn quit_command_returns_quit_outcome() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();
    let outcome = run_command(&mut app, &mut io, "q");
    assert_eq!(outcome, CommandOutcome::Quit);
}
