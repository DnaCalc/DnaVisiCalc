use std::fs;
use std::path::Path;

use dnavisicalc_tui::{Action, App, MemoryWorkbookIo, render_app};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("artifacts/readme/scenes");
    fs::create_dir_all(out_dir)?;

    capture_startup(out_dir.join("01_startup.txt"))?;
    capture_editing(out_dir.join("02_editing.txt"))?;
    capture_help(out_dir.join("03_help_popup.txt"))?;
    capture_command(out_dir.join("04_command_mode.txt"))?;
    capture_numerical_model(out_dir.join("05_numerical_model.txt"))?;

    Ok(())
}

fn capture_startup(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new();
    write_scene(&app, path)
}

fn capture_editing(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    app.apply(Action::StartCommand, &mut io);
    for ch in "set A1 DNA".chars() {
        app.apply(Action::InputChar(ch), &mut io);
    }
    app.apply(Action::Submit, &mut io);

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::StartEdit, &mut io);
    for ch in "=A1&\" VisiCalc\"".chars() {
        app.apply(Action::InputChar(ch), &mut io);
    }
    app.apply(Action::Submit, &mut io);

    write_scene(&app, path)
}

fn capture_help(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();
    app.apply(Action::ToggleHelp, &mut io);
    write_scene(&app, path)
}

fn capture_command(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();
    apply_command(&mut app, &mut io, "mode manual");
    write_scene(&app, path)
}

fn capture_numerical_model(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Price",
        "set B1 Qty",
        "set C1 Revenue",
        "set D1 Growth%",
        "set E1 PMT",
        "set A2 12.5",
        "set B2 48",
        "set C2 =ROUND(A2*B2,2)",
        "set A3 13.4",
        "set B3 54",
        "set C3 =ROUND(A3*B3,2)",
        "set D3 =ROUND((C3-C2)/C2*100,2)",
        "set E3 =ROUND(PMT(0.05/12,360,300000),2)",
        "set F1 NPV",
        "set F3 =ROUND(NPV(0.1,C2:C3),2)",
    ];

    for command in commands {
        apply_command(&mut app, &mut io, command);
    }

    for _ in 0..4 {
        app.apply(Action::MoveDown, &mut io);
    }
    for _ in 0..4 {
        app.apply(Action::MoveRight, &mut io);
    }

    write_scene(&app, path)
}

fn apply_command(app: &mut App, io: &mut MemoryWorkbookIo, command: &str) {
    app.apply(Action::StartCommand, io);
    for ch in command.chars() {
        app.apply(Action::InputChar(ch), io);
    }
    app.apply(Action::Submit, io);
}

fn write_scene(app: &App, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_app(frame, app))?;
    let buffer = terminal.backend().buffer();

    let width = buffer.area().width as usize;
    let text = buffer
        .content()
        .chunks(width)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(path, text)?;
    Ok(())
}
