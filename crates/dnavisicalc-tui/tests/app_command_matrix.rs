use dnavisicalc_core::RecalcMode;
use dnavisicalc_tui::{Action, App, CommandOutcome, MemoryWorkbookIo};

fn run_command(app: &mut App, io: &mut MemoryWorkbookIo, command: &str) -> CommandOutcome {
    app.apply(Action::StartCommand, io);
    for ch in command.chars() {
        app.apply(Action::InputChar(ch), io);
    }
    app.apply(Action::Submit, io)
}

#[test]
fn command_usage_errors_are_reported() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "w");
    assert!(app.status().contains("Usage: w"));

    run_command(&mut app, &mut io, "o");
    assert!(app.status().contains("Usage: o"));

    run_command(&mut app, &mut io, "set");
    assert!(app.status().contains("Usage: set"));

    run_command(&mut app, &mut io, "mode");
    assert!(app.status().contains("Usage: mode"));
}

#[test]
fn mode_command_switches_engine_modes() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "mode manual");
    assert_eq!(app.engine().recalc_mode(), RecalcMode::Manual);

    run_command(&mut app, &mut io, "mode auto");
    assert_eq!(app.engine().recalc_mode(), RecalcMode::Automatic);
}

#[test]
fn unknown_and_help_commands_set_status() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "help");
    assert!(app.status().contains("Help shown"));

    run_command(&mut app, &mut io, "nope");
    assert!(app.status().contains("Unknown command"));
}

#[test]
fn cancel_in_command_and_edit_returns_to_navigation() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    app.apply(Action::StartCommand, &mut io);
    app.apply(Action::Cancel, &mut io);
    assert_eq!(app.mode(), dnavisicalc_tui::AppMode::Navigate);

    app.apply(Action::StartEdit, &mut io);
    app.apply(Action::Cancel, &mut io);
    assert_eq!(app.mode(), dnavisicalc_tui::AppMode::Navigate);
}

#[test]
fn empty_and_invalid_mode_commands_report_status() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "");
    assert!(app.status().contains("No command"));

    run_command(&mut app, &mut io, "mode maybe");
    assert!(app.status().contains("Usage: mode"));
}

#[test]
fn write_without_path_uses_last_saved_path() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "set A1 5");
    run_command(&mut app, &mut io, "w remember.dvc");
    assert!(io.files().contains_key("remember.dvc"));

    run_command(&mut app, &mut io, "set A1 7");
    run_command(&mut app, &mut io, "w");

    let reloaded =
        dnavisicalc_file::load_from_str(io.files().get("remember.dvc").expect("saved content"))
            .expect("load remembered file");
    assert_eq!(
        reloaded.cell_state_a1("A1").expect("query").value,
        dnavisicalc_core::Value::Number(7.0)
    );
}

#[test]
fn set_command_accepts_text_payload() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "set A1 hello");
    assert!(app.status().contains("Set A1"));
    assert_eq!(
        app.engine().cell_state_a1("A1").expect("A1").value,
        dnavisicalc_core::Value::Text("hello".to_string())
    );
}

#[test]
fn name_command_sets_and_clears_names() {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    run_command(&mut app, &mut io, "name rate 0.2");
    assert!(app.status().contains("Set name RATE"));

    run_command(&mut app, &mut io, "set A1 100");
    run_command(&mut app, &mut io, "name total =A1*(1+RATE)");
    run_command(&mut app, &mut io, "set B1 =TOTAL");
    assert_eq!(
        app.engine().cell_state_a1("B1").expect("B1").value,
        dnavisicalc_core::Value::Number(120.0)
    );

    run_command(&mut app, &mut io, "name clear rate");
    assert!(app.status().contains("Cleared name RATE"));
}
