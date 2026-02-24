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
    app.apply(Action::StartCommand, &mut io);
    for ch in "mode manual".chars() {
        app.apply(Action::InputChar(ch), &mut io);
    }
    write_scene(&app, path)
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
