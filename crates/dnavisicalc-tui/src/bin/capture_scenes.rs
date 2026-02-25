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
    capture_names_model(out_dir.join("06_names_model.txt"))?;
    capture_paste_special_picker(out_dir.join("07_paste_special_picker.txt"))?;
    capture_paste_special_result(out_dir.join("08_paste_special_result.txt"))?;
    capture_formatting_and_colors(out_dir.join("09_formatting_colors.txt"))?;
    capture_dynamic_arrays(out_dir.join("10_dynamic_arrays.txt"))?;

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
        "set F1 NPV_10%",
        "set F3 =ROUND(NPV(0.1,C2:C3),2)",
        "set G1 Notes",
        "set G2 C2=ROUND(A2*B2,2)",
        "set G3 D3=ROUND((C3-C2)/C2*100,2)",
    ];

    apply_commands(&mut app, &mut io, &commands);
    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::MoveDown, &mut io);

    write_scene(&app, path)
}

fn capture_names_model(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "name TAX_RATE 0.21",
        "name DISCOUNT 0.05",
        "set A1 Base",
        "set B1 Gross",
        "set C1 Tax",
        "set D1 AfterDisc",
        "set A2 100",
        "set B2 =ROUND(A2*(1+TAX_RATE),2)",
        "set C2 =ROUND(B2-A2,2)",
        "set D2 =ROUND(B2*(1-DISCOUNT),2)",
        "set A3 250",
        "set B3 =ROUND(A3*(1+TAX_RATE),2)",
        "set C3 =ROUND(B3-A3,2)",
        "set D3 =ROUND(B3*(1-DISCOUNT),2)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::MoveDown, &mut io);

    write_scene(&app, path)
}

fn capture_paste_special_picker(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Product",
        "set B1 Units",
        "set A2 DNA",
        "set B2 =LEN(A2)*10",
    ];
    apply_commands(&mut app, &mut io, &commands);

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::ExtendLeft, &mut io);
    app.apply(Action::ExtendUp, &mut io);
    app.apply(Action::CopySelection, &mut io);

    let clipboard_text = app
        .last_copy_text()
        .map(ToString::to_string)
        .unwrap_or_else(|| "DNA\t30".to_string());

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::BeginPasteFromClipboard(clipboard_text), &mut io);
    app.apply(Action::InputChar('2'), &mut io);

    write_scene(&app, path)
}

fn capture_paste_special_result(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = ["set A1 1", "set B1 =A1+10", "set A2 2", "set B2 =A2+10"];
    apply_commands(&mut app, &mut io, &commands);

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::ExtendLeft, &mut io);
    app.apply(Action::ExtendUp, &mut io);
    app.apply(Action::CopySelection, &mut io);
    let clipboard_text = app
        .last_copy_text()
        .map(ToString::to_string)
        .unwrap_or_else(|| "1\t11\n2\t12".to_string());

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::BeginPasteFromClipboard(clipboard_text), &mut io);
    app.apply(Action::InputChar('4'), &mut io);
    app.apply(Action::Submit, &mut io);

    write_scene(&app, path)
}

fn capture_formatting_and_colors(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Forest",
        "set B1 =A1&\" theme\"",
        "set A2 3.14159",
        "set B2 =A2*2",
    ];
    apply_commands(&mut app, &mut io, &commands);

    app.apply(Action::MoveRight, &mut io);
    app.apply(Action::MoveDown, &mut io);
    app.apply(Action::ExtendLeft, &mut io);
    app.apply(Action::ExtendUp, &mut io);
    apply_commands(
        &mut app,
        &mut io,
        &[
            "fmt bold on",
            "fmt italic on",
            "fmt fg fern",
            "fmt bg sand",
            "fmt decimals 2",
        ],
    );

    write_scene(&app, path)
}

fn capture_dynamic_arrays(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 =SEQUENCE(4,3,1,1)",
        "set E1 =RANDARRAY(4,2,10,99,TRUE)",
        "set H1 =SUM(A1#)",
        "set H2 =AVERAGE(A1#)",
        "set H3 =MAX(A1#)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    write_scene(&app, path)
}

fn apply_commands(app: &mut App, io: &mut MemoryWorkbookIo, commands: &[&str]) {
    for command in commands {
        apply_command(app, io, command);
    }
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
