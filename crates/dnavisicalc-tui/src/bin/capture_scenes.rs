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
    capture_bioreactor_dashboard(out_dir.join("11_bioreactor_dashboard.txt"))?;
    capture_palette_showcase(out_dir.join("12_palette_showcase.txt"))?;
    capture_names_let_lambda(out_dir.join("13_names_let_lambda.txt"))?;
    capture_indirect_r1c1_offset(out_dir.join("14_indirect_r1c1_offset.txt"))?;
    capture_map_array_tiles(out_dir.join("15_map_array_tiles.txt"))?;
    capture_dynamic_array_lab(out_dir.join("16_dynamic_array_lab.txt"))?;

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

fn capture_bioreactor_dashboard(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "name TAX_RATE 0.19",
        "name TARGET_YIELD 0.92",
        "set A1 Batch",
        "set B1 Feed_l",
        "set C1 Output_l",
        "set D1 Yield_%",
        "set E1 Cost",
        "set F1 Margin",
        "set G1 Decision",
        "set A2 B-201",
        "set B2 1200",
        "set C2 1110",
        "set D2 =ROUND(C2/B2*100,2)",
        "set E2 4200",
        "set F2 =ROUND((C2*6.2-E2)*(1-TAX_RATE),2)",
        "set G2 =IF(D2>=TARGET_YIELD*100,\"GO\",\"Tune\")",
        "set A3 B-202",
        "set B3 1260",
        "set C3 1188",
        "set D3 =ROUND(C3/B3*100,2)",
        "set E3 4360",
        "set F3 =ROUND((C3*6.2-E3)*(1-TAX_RATE),2)",
        "set G3 =IF(D3>=TARGET_YIELD*100,\"GO\",\"Tune\")",
        "set A4 B-203",
        "set B4 1280",
        "set C4 1150",
        "set D4 =ROUND(C4/B4*100,2)",
        "set E4 4425",
        "set F4 =ROUND((C4*6.2-E4)*(1-TAX_RATE),2)",
        "set G4 =IF(D4>=TARGET_YIELD*100,\"GO\",\"Tune\")",
        "set A5 B-204",
        "set B5 1310",
        "set C5 1230",
        "set D5 =ROUND(C5/B5*100,2)",
        "set E5 4510",
        "set F5 =ROUND((C5*6.2-E5)*(1-TAX_RATE),2)",
        "set G5 =IF(D5>=TARGET_YIELD*100,\"GO\",\"Tune\")",
        "set I1 Formulas",
        "set I2 D2=ROUND(C2/B2*100,2)",
        "set I3 F2=ROUND((C2*6.2-E2)*(1-TAX_RATE),2)",
        "set I4 G2=IF(D2>=TARGET_YIELD*100,\"GO\",\"Tune\")",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 1, 1, 7, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg lagoon"],
    );
    select_rect(&mut app, &mut io, 2, 4, 6, 5);
    apply_commands(&mut app, &mut io, &["fmt decimals 2", "fmt fg fern"]);
    select_rect(&mut app, &mut io, 4, 2, 4, 5);
    apply_commands(&mut app, &mut io, &["fmt bg seafoam"]);
    select_rect(&mut app, &mut io, 7, 2, 7, 5);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg peach", "fmt bg moss"],
    );

    select_rect(&mut app, &mut io, 3, 3, 3, 3);
    write_scene(&app, path)
}

fn capture_palette_showcase(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Palette",
        "set B1 Theme",
        "set C1 Example",
        "set D1 Score",
        "set A2 Mist",
        "set B2 Lagoon",
        "set C2 Forest sample",
        "set D2 82.1234",
        "set A3 Sage",
        "set B3 Teal",
        "set C3 Ocean sample",
        "set D3 78.9578",
        "set A4 Fern",
        "set B4 Clay",
        "set C4 Sunset sample",
        "set D4 91.3478",
        "set A5 Rose",
        "set B5 Sky",
        "set C5 Orchid sample",
        "set D5 88.7777",
        "set F1 Formatting",
        "set F2 Bold+Italic text, decimals=1, fg/bg mixed per column",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 1, 1, 4, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );
    select_rect(&mut app, &mut io, 1, 2, 1, 5);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);
    select_rect(&mut app, &mut io, 2, 2, 2, 5);
    apply_commands(&mut app, &mut io, &["fmt fg sky", "fmt bg sand"]);
    select_rect(&mut app, &mut io, 3, 2, 3, 5);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt italic on", "fmt fg rose"]);
    select_rect(&mut app, &mut io, 4, 2, 4, 5);
    apply_commands(&mut app, &mut io, &["fmt decimals 1", "fmt fg cloud", "fmt bg lagoon"]);

    select_rect(&mut app, &mut io, 3, 4, 3, 4);
    write_scene(&app, path)
}

fn capture_names_let_lambda(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "name BASE_RATE 0.075",
        "name RISK_ADJ 1.12",
        "set A1 Segment",
        "set B1 Principal",
        "set C1 Months",
        "set D1 Payment",
        "set E1 Stress",
        "set A2 Core",
        "set B2 120000",
        "set C2 24",
        "set D2 =ROUND(PMT(BASE_RATE/12,C2,B2),2)",
        "set E2 =ROUND(LET(p,D2,m,C2,p*m*RISK_ADJ),2)",
        "set A3 Growth",
        "set B3 98000",
        "set C3 18",
        "set D3 =ROUND(PMT(BASE_RATE/12,C3,B3),2)",
        "set E3 =ROUND(LET(p,D3,m,C3,p*m*RISK_ADJ),2)",
        "set G1 Lambda",
        "set G2 =LET(scale,LAMBDA(x,x*RISK_ADJ),scale(D2))",
        "set G3 =LET(scale,LAMBDA(x,x*RISK_ADJ),scale(D3))",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 1, 1, 5, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );
    select_rect(&mut app, &mut io, 4, 2, 5, 3);
    apply_commands(&mut app, &mut io, &["fmt decimals 2", "fmt fg peach"]);
    select_rect(&mut app, &mut io, 7, 1, 7, 3);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg lavender", "fmt bg olive"],
    );

    select_rect(&mut app, &mut io, 7, 2, 7, 2);
    write_scene(&app, path)
}

fn capture_indirect_r1c1_offset(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 15",
        "set A2 21",
        "set B2 34",
        "set B3 55",
        "set D1 =INDIRECT(\"R2C2\",FALSE)",
        "set D2 =INDIRECT(\"R[-1]C[-3]\",FALSE)",
        "set D3 =SUM(INDIRECT(\"R2C2:R3C2\",FALSE))",
        "set D4 =OFFSET(A1,1,1,2,1)",
        "set F1 Notes",
        "set F2 D1 absolute R1C1",
        "set F3 D2 relative R1C1",
        "set F4 D3 sum over R1C1 range",
        "set F5 D4 top-left of OFFSET range",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 4, 1, 4, 4);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg moss"],
    );
    select_rect(&mut app, &mut io, 6, 1, 6, 5);
    apply_commands(&mut app, &mut io, &["fmt fg sky"]);

    select_rect(&mut app, &mut io, 4, 3, 4, 3);
    write_scene(&app, path)
}

fn capture_map_array_tiles(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 1",
        "set A2 2",
        "set A3 3",
        "set C1 =MAP(A1:A3,LAMBDA(x,SEQUENCE(1,3,x,1)))",
        "set G1 =MAP(A1:A3,LAMBDA(x,SEQUENCE(2,1,x,10)))",
        "set J1 Formula",
        "set J2 C1=MAP(...,LAMBDA(x,SEQUENCE(1,3,x,1)))",
        "set J3 G1=MAP(...,LAMBDA(x,SEQUENCE(2,1,x,10)))",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 3, 1, 5, 3);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);
    select_rect(&mut app, &mut io, 7, 1, 7, 6);
    apply_commands(&mut app, &mut io, &["fmt fg peach", "fmt bg seafoam"]);
    select_rect(&mut app, &mut io, 10, 1, 10, 3);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt fg cloud", "fmt bg slate"]);

    select_rect(&mut app, &mut io, 3, 1, 3, 1);
    write_scene(&app, path)
}

fn capture_dynamic_array_lab(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 =SEQUENCE(6,3,10,2)",
        "set E1 =RANDARRAY(6,3,0,1,FALSE)",
        "set I1 =SUM(A1#)",
        "set I2 =AVERAGE(A1#)",
        "set I3 =MAX(A1#)",
        "set I4 =MIN(A1#)",
        "set K1 Dashboard",
        "set K2 A1# deterministic sequence",
        "set K3 E1# random 6x3",
        "set K4 I1:I4 aggregate stats",
    ];
    apply_commands(&mut app, &mut io, &commands);

    select_rect(&mut app, &mut io, 1, 1, 3, 6);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);
    select_rect(&mut app, &mut io, 5, 1, 7, 6);
    apply_commands(&mut app, &mut io, &["fmt fg lavender", "fmt bg cloud"]);
    select_rect(&mut app, &mut io, 9, 1, 9, 4);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt decimals 2", "fmt fg cloud", "fmt bg lagoon"],
    );

    select_rect(&mut app, &mut io, 9, 2, 9, 2);
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

fn select_rect(
    app: &mut App,
    io: &mut MemoryWorkbookIo,
    col_start: u16,
    row_start: u16,
    col_end: u16,
    row_end: u16,
) {
    move_to_cell(app, io, col_start, row_start);
    for _ in col_start..col_end {
        app.apply(Action::ExtendRight, io);
    }
    for _ in row_start..row_end {
        app.apply(Action::ExtendDown, io);
    }
}

fn move_to_cell(app: &mut App, io: &mut MemoryWorkbookIo, col: u16, row: u16) {
    for _ in 0..80 {
        app.apply(Action::MoveLeft, io);
    }
    for _ in 0..300 {
        app.apply(Action::MoveUp, io);
    }
    for _ in 1..col {
        app.apply(Action::MoveRight, io);
    }
    for _ in 1..row {
        app.apply(Action::MoveDown, io);
    }
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
