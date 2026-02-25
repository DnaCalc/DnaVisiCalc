use std::fs;
use std::path::Path;

use dnavisicalc_tui::{Action, App, MemoryWorkbookIo, render_app};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

struct StyledSpan {
    text: String,
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    italic: bool,
}

fn color_to_hex(color: Color) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Black => Some("#000000".to_string()),
        Color::Red => Some("#FF0000".to_string()),
        Color::Green => Some("#00FF00".to_string()),
        Color::Yellow => Some("#FFFF00".to_string()),
        Color::Blue => Some("#0000FF".to_string()),
        Color::Magenta => Some("#FF00FF".to_string()),
        Color::Cyan => Some("#00FFFF".to_string()),
        Color::Gray => Some("#808080".to_string()),
        Color::DarkGray => Some("#A0A0A0".to_string()),
        Color::LightRed => Some("#FF8080".to_string()),
        Color::LightGreen => Some("#80FF80".to_string()),
        Color::LightYellow => Some("#FFFF80".to_string()),
        Color::LightBlue => Some("#8080FF".to_string()),
        Color::LightMagenta => Some("#FF80FF".to_string()),
        Color::LightCyan => Some("#80FFFF".to_string()),
        Color::White => Some("#FFFFFF".to_string()),
        Color::Rgb(r, g, b) => Some(format!("#{:02X}{:02X}{:02X}", r, g, b)),
        Color::Indexed(i) => Some(format!("#{0:02X}{0:02X}{0:02X}", i)),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("artifacts/readme/scenes");
    fs::create_dir_all(out_dir)?;

    capture_startup(out_dir.join("01_startup.txt"))?;
    capture_editing(out_dir.join("02_editing.txt"))?;
    capture_help(out_dir.join("03_help_popup.txt"))?;
    capture_command(out_dir.join("04_command_mode.txt"))?;
    capture_multiplication_table(out_dir.join("05_multiplication_table.txt"))?;
    capture_scientific_calculator(out_dir.join("06_scientific_calculator.txt"))?;
    capture_financial_model(out_dir.join("07_financial_model.txt"))?;
    capture_names_tax_model(out_dir.join("08_names_tax_model.txt"))?;
    capture_loan_calculator(out_dir.join("09_loan_calculator.txt"))?;
    capture_formatting_showcase(out_dir.join("10_formatting_showcase.txt"))?;
    capture_full_palette(out_dir.join("11_full_palette.txt"))?;
    capture_sequence_aggregates(out_dir.join("12_sequence_aggregates.txt"))?;
    capture_randarray_lab(out_dir.join("13_randarray_lab.txt"))?;
    capture_map_array_tiles(out_dir.join("14_map_array_tiles.txt"))?;
    capture_let_lambda(out_dir.join("15_let_lambda.txt"))?;
    capture_indirect_r1c1_offset(out_dir.join("16_indirect_r1c1_offset.txt"))?;
    capture_lookup_model(out_dir.join("17_lookup_model.txt"))?;
    capture_student_gradebook(out_dir.join("18_student_gradebook.txt"))?;
    capture_text_functions(out_dir.join("19_text_functions.txt"))?;
    capture_paste_special_picker(out_dir.join("20_paste_special_picker.txt"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Scene writer (txt + json)
// ---------------------------------------------------------------------------

fn write_scene(app: &App, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_app(frame, app))?;
    let buffer = terminal.backend().buffer();

    let width = buffer.area().width as usize;
    let height = buffer.area().height as usize;

    // ---- .txt (existing logic) ----
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
    fs::write(&path, &text)?;

    // ---- .json with style data ----
    let json_path = path.as_ref().with_extension("json");
    let content = buffer.content();
    let mut json = format!("{{\"width\":{},\"height\":{},\"rows\":[\n", width, height);

    for y in 0..height {
        if y > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{{\"y\":{},\"spans\":[", y));

        let row_start = y * width;
        let mut spans: Vec<StyledSpan> = Vec::new();

        for x in 0..width {
            let cell = &content[row_start + x];
            let fg = color_to_hex(cell.fg);
            let bg = color_to_hex(cell.bg);
            let bold = cell.modifier.contains(Modifier::BOLD);
            let italic = cell.modifier.contains(Modifier::ITALIC);
            let sym = cell.symbol();

            if let Some(last) = spans.last_mut() {
                if last.fg == fg && last.bg == bg && last.bold == bold && last.italic == italic {
                    last.text.push_str(sym);
                    continue;
                }
            }
            spans.push(StyledSpan {
                text: sym.to_string(),
                fg,
                bg,
                bold,
                italic,
            });
        }

        for (i, span) in spans.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let fg_json = match &span.fg {
                Some(c) => format!("\"{}\"", c),
                None => "null".to_string(),
            };
            let bg_json = match &span.bg {
                Some(c) => format!("\"{}\"", c),
                None => "null".to_string(),
            };
            json.push_str(&format!(
                "{{\"text\":\"{}\",\"fg\":{},\"bg\":{},\"bold\":{},\"italic\":{}}}",
                json_escape(&span.text),
                fg_json,
                bg_json,
                span.bold,
                span.italic,
            ));
        }
        json.push_str("]}");
    }

    json.push_str("\n]}");
    fs::write(json_path, &json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Getting Started (scenes 01-04)
// ---------------------------------------------------------------------------

fn capture_startup(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new();
    write_scene(&app, path)
}

fn capture_editing(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    apply_commands(
        &mut app,
        &mut io,
        &["set A1 DNA", "set B1 =A1&\" VisiCalc\"", "set A2 42"],
    );

    // Format header row: bold + Cloud fg + Slate bg
    select_rect(&mut app, &mut io, 1, 1, 2, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );

    // Navigate to B2 and start editing (don't submit — show edit mode)
    move_to_cell(&mut app, &mut io, 2, 2);
    app.apply(Action::StartEdit, &mut io);
    for ch in "=A2*2".chars() {
        app.apply(Action::InputChar(ch), &mut io);
    }

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
    // Show command being typed (not submitted)
    app.apply(Action::StartCommand, &mut io);
    for ch in "mode manual".chars() {
        app.apply(Action::InputChar(ch), &mut io);
    }
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Math & Science (scenes 05-06)
// ---------------------------------------------------------------------------

fn capture_multiplication_table(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let cols = ['B', 'C', 'D', 'E', 'F', 'G', 'H', 'I'];
    let mut commands: Vec<String> = Vec::new();

    commands.push("set A1 x".to_string());
    for (i, &col) in cols.iter().enumerate() {
        commands.push(format!("set {}1 {}", col, i + 1));
    }
    for r in 2..=9u16 {
        commands.push(format!("set A{} {}", r, r - 1));
    }
    for &col in &cols {
        for r in 2..=9u16 {
            commands.push(format!("set {}{} =A{}*{}1", col, r, r, col));
        }
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    apply_commands(&mut app, &mut io, &cmd_refs);

    // Headers: bold + Cloud fg + Teal bg
    select_rect(&mut app, &mut io, 1, 1, 9, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );
    select_rect(&mut app, &mut io, 1, 2, 1, 9);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Rainbow column colors for data
    let col_colors = [
        "fern", "sky", "peach", "lavender", "rose", "seafoam", "sand", "moss",
    ];
    for (i, color) in col_colors.iter().enumerate() {
        let col = (i + 2) as u16;
        select_rect(&mut app, &mut io, col, 2, col, 9);
        apply_command(&mut app, &mut io, &format!("fmt fg {}", color));
    }

    // Decimals 0
    select_rect(&mut app, &mut io, 2, 2, 9, 9);
    apply_commands(&mut app, &mut io, &["fmt decimals 0"]);

    select_rect(&mut app, &mut io, 5, 5, 5, 5);
    write_scene(&app, path)
}

fn capture_scientific_calculator(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let mut commands: Vec<String> = Vec::new();
    commands.push("set A1 Angle".to_string());
    commands.push("set B1 SIN".to_string());
    commands.push("set C1 COS".to_string());
    commands.push("set D1 TAN".to_string());
    commands.push("set E1 SQRT".to_string());
    commands.push("set F1 EXP".to_string());
    commands.push("set G1 LN".to_string());

    let angles = [0, 15, 30, 45, 60, 75, 90];
    for (i, &angle) in angles.iter().enumerate() {
        let r = i + 2;
        commands.push(format!("set A{} {}", r, angle));
        commands.push(format!("set B{} =ROUND(SIN(A{}*PI()/180),4)", r, r));
        commands.push(format!("set C{} =ROUND(COS(A{}*PI()/180),4)", r, r));
        commands.push(format!("set D{} =ROUND(TAN(A{}*PI()/180),4)", r, r));
        commands.push(format!("set E{} =ROUND(SQRT(A{}),4)", r, r));
        commands.push(format!("set F{} =ROUND(EXP(A{}/30),4)", r, r));
        commands.push(format!("set G{} =ROUND(LN(A{}+1),4)", r, r));
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    apply_commands(&mut app, &mut io, &cmd_refs);

    // Headers: bold + Cloud + Teal
    select_rect(&mut app, &mut io, 1, 1, 7, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Each function column a different fg
    let col_colors: [(u16, &str); 6] = [
        (2, "fern"),
        (3, "sky"),
        (4, "peach"),
        (5, "lavender"),
        (6, "rose"),
        (7, "seafoam"),
    ];
    for (col, color) in &col_colors {
        select_rect(&mut app, &mut io, *col, 2, *col, 8);
        apply_command(&mut app, &mut io, &format!("fmt fg {}", color));
    }

    // Decimals 4
    select_rect(&mut app, &mut io, 2, 2, 7, 8);
    apply_commands(&mut app, &mut io, &["fmt decimals 4"]);

    select_rect(&mut app, &mut io, 3, 4, 3, 4);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Financial (scenes 07-09)
// ---------------------------------------------------------------------------

fn capture_financial_model(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Price",
        "set B1 Qty",
        "set C1 Revenue",
        "set D1 Growth%",
        "set E1 PMT",
        "set F1 NPV_10%",
        "set A2 12.50",
        "set B2 48",
        "set C2 =ROUND(A2*B2,2)",
        "set D2 -",
        "set E2 =ROUND(PMT(0.05/12,360,200000),2)",
        "set F2 -",
        "set A3 13.40",
        "set B3 54",
        "set C3 =ROUND(A3*B3,2)",
        "set D3 =ROUND((C3-C2)/C2*100,2)",
        "set E3 =ROUND(PMT(0.05/12,360,250000),2)",
        "set F3 =ROUND(NPV(0.1,C2:C3),2)",
        "set A4 15.20",
        "set B4 61",
        "set C4 =ROUND(A4*B4,2)",
        "set D4 =ROUND((C4-C3)/C3*100,2)",
        "set E4 =ROUND(PMT(0.05/12,360,300000),2)",
        "set F4 =ROUND(NPV(0.1,C2:C4),2)",
        "set A5 14.80",
        "set B5 58",
        "set C5 =ROUND(A5*B5,2)",
        "set D5 =ROUND((C5-C4)/C4*100,2)",
        "set E5 =ROUND(PMT(0.05/12,360,350000),2)",
        "set F5 =ROUND(NPV(0.1,C2:C5),2)",
        "set A6 16.90",
        "set B6 72",
        "set C6 =ROUND(A6*B6,2)",
        "set D6 =ROUND((C6-C5)/C5*100,2)",
        "set E6 =ROUND(PMT(0.05/12,360,400000),2)",
        "set F6 =ROUND(NPV(0.1,C2:C6),2)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Headers: bold + Cloud + Teal
    select_rect(&mut app, &mut io, 1, 1, 6, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Revenue: fg Fern + bg Mist
    select_rect(&mut app, &mut io, 3, 2, 3, 6);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);

    // PMT: bold + Lavender + Olive
    select_rect(&mut app, &mut io, 5, 2, 5, 6);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg lavender", "fmt bg olive"],
    );

    select_rect(&mut app, &mut io, 3, 3, 3, 3);
    write_scene(&app, path)
}

fn capture_names_tax_model(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "name TAX_RATE 0.21",
        "name DISCOUNT 0.05",
        "set A1 Base",
        "set B1 Gross",
        "set C1 Tax",
        "set D1 Net",
        "set A2 100",
        "set B2 =ROUND(A2*(1+TAX_RATE),2)",
        "set C2 =ROUND(B2-A2,2)",
        "set D2 =ROUND(B2*(1-DISCOUNT),2)",
        "set A3 250",
        "set B3 =ROUND(A3*(1+TAX_RATE),2)",
        "set C3 =ROUND(B3-A3,2)",
        "set D3 =ROUND(B3*(1-DISCOUNT),2)",
        "set A4 500",
        "set B4 =ROUND(A4*(1+TAX_RATE),2)",
        "set C4 =ROUND(B4-A4,2)",
        "set D4 =ROUND(B4*(1-DISCOUNT),2)",
        "set A5 1000",
        "set B5 =ROUND(A5*(1+TAX_RATE),2)",
        "set C5 =ROUND(B5-A5,2)",
        "set D5 =ROUND(B5*(1-DISCOUNT),2)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Headers: bold + Cloud + Lagoon
    select_rect(&mut app, &mut io, 1, 1, 4, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg lagoon"],
    );

    // Tax: fg Rose + bg Sand
    select_rect(&mut app, &mut io, 3, 2, 3, 5);
    apply_commands(&mut app, &mut io, &["fmt fg rose", "fmt bg sand"]);

    select_rect(&mut app, &mut io, 2, 3, 2, 3);
    write_scene(&app, path)
}

fn capture_loan_calculator(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "name RATE 0.065",
        "name TERM 360",
        "set A1 Loan",
        "set B1 Princpal",
        "set C1 PMT",
        "set D1 PV",
        "set E1 FV",
        "set F1 NPV",
        "set A2 Home",
        "set B2 250000",
        "set C2 =ROUND(PMT(RATE/12,TERM,B2),2)",
        "set D2 =ROUND(PV(RATE/12,TERM,C2),2)",
        "set E2 =ROUND(FV(RATE/12,TERM,C2),2)",
        "set F2 =ROUND(NPV(RATE,C2),2)",
        "set A3 Auto",
        "set B3 35000",
        "set C3 =ROUND(PMT(RATE/12,60,B3),2)",
        "set D3 =ROUND(PV(RATE/12,60,C3),2)",
        "set E3 =ROUND(FV(RATE/12,60,C3),2)",
        "set F3 =ROUND(NPV(RATE,C3),2)",
        "set A4 Student",
        "set B4 45000",
        "set C4 =ROUND(PMT(RATE/12,120,B4),2)",
        "set D4 =ROUND(PV(RATE/12,120,C4),2)",
        "set E4 =ROUND(FV(RATE/12,120,C4),2)",
        "set F4 =ROUND(NPV(RATE,C4),2)",
        "set A5 Business",
        "set B5 150000",
        "set C5 =ROUND(PMT(RATE/12,240,B5),2)",
        "set D5 =ROUND(PV(RATE/12,240,C5),2)",
        "set E5 =ROUND(FV(RATE/12,240,C5),2)",
        "set F5 =ROUND(NPV(RATE,C5),2)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Headers: bold + Cloud + Moss
    select_rect(&mut app, &mut io, 1, 1, 6, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg moss"],
    );

    // Results: fg Peach + decimals 2
    select_rect(&mut app, &mut io, 3, 2, 6, 5);
    apply_commands(&mut app, &mut io, &["fmt fg peach", "fmt decimals 2"]);

    select_rect(&mut app, &mut io, 3, 2, 3, 2);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Formatting & Palette (scenes 10-11)
// ---------------------------------------------------------------------------

fn capture_formatting_showcase(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 Bold",
        "set B1 Italic",
        "set C1 Bold+Ital",
        "set D1 Dec=2",
        "set A2 Forest",
        "set A3 Ocean",
        "set A4 Sunset",
        "set A5 Galaxy",
        "set B2 Forest",
        "set B3 Ocean",
        "set B4 Sunset",
        "set B5 Galaxy",
        "set C2 Forest",
        "set C3 Ocean",
        "set C4 Sunset",
        "set C5 Galaxy",
        "set D2 3.14159",
        "set D3 2.71828",
        "set D4 1.41421",
        "set D5 1.61803",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Headers
    select_rect(&mut app, &mut io, 1, 1, 4, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );

    // Col A: Bold, fg Fern, bg Mist
    select_rect(&mut app, &mut io, 1, 2, 1, 5);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg fern", "fmt bg mist"],
    );

    // Col B: Italic, fg Sky, bg Sand
    select_rect(&mut app, &mut io, 2, 2, 2, 5);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt italic on", "fmt fg sky", "fmt bg sand"],
    );

    // Col C: Bold+Italic, fg Rose, bg Seafoam
    select_rect(&mut app, &mut io, 3, 2, 3, 5);
    apply_commands(
        &mut app,
        &mut io,
        &[
            "fmt bold on",
            "fmt italic on",
            "fmt fg rose",
            "fmt bg seafoam",
        ],
    );

    // Col D: Decimals=2, fg Lavender, bg Cloud
    select_rect(&mut app, &mut io, 4, 2, 4, 5);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt decimals 2", "fmt fg lavender", "fmt bg cloud"],
    );

    select_rect(&mut app, &mut io, 2, 3, 2, 3);
    write_scene(&app, path)
}

fn capture_full_palette(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let colors = [
        "Mist", "Sage", "Fern", "Moss", "Olive", "Seafoam", "Lagoon", "Teal", "Sky", "Cloud",
        "Sand", "Clay", "Peach", "Rose", "Lavender", "Slate",
    ];

    let mut commands: Vec<String> = Vec::new();
    commands.push("set A1 Color".to_string());
    commands.push("set B1 Foreground".to_string());
    commands.push("set C1 Background".to_string());
    commands.push("set D1 Number".to_string());

    for (i, name) in colors.iter().enumerate() {
        let r = i + 2;
        commands.push(format!("set A{} {}", r, name));
        commands.push(format!("set B{} Sample", r));
        commands.push(format!("set C{} Sample", r));
        commands.push(format!("set D{} {}.{:02}", r, (i + 1) * 7, (i * 37) % 100));
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    apply_commands(&mut app, &mut io, &cmd_refs);

    // Headers
    select_rect(&mut app, &mut io, 1, 1, 4, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );

    // Per-color formatting
    for (i, name) in colors.iter().enumerate() {
        let row = (i + 2) as u16;
        let lower = name.to_lowercase();

        // Col B: this color as fg
        select_rect(&mut app, &mut io, 2, row, 2, row);
        apply_command(&mut app, &mut io, &format!("fmt fg {}", lower));

        // Col C: this color as bg, Cloud fg
        select_rect(&mut app, &mut io, 3, row, 3, row);
        apply_command(&mut app, &mut io, &format!("fmt bg {}", lower));
        apply_command(&mut app, &mut io, "fmt fg cloud");

        // Col D: bold + this color as fg
        select_rect(&mut app, &mut io, 4, row, 4, row);
        apply_command(&mut app, &mut io, &format!("fmt fg {}", lower));
        apply_command(&mut app, &mut io, "fmt bold on");
    }

    select_rect(&mut app, &mut io, 2, 5, 2, 5);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Dynamic Arrays & Spill (scenes 12-14)
// ---------------------------------------------------------------------------

fn capture_sequence_aggregates(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 =SEQUENCE(6,3,1,1)",
        "set E1 Stat",
        "set F1 Value",
        "set E2 SUM",
        "set F2 =SUM(A1#)",
        "set E3 AVG",
        "set F3 =AVERAGE(A1#)",
        "set E4 MAX",
        "set F4 =MAX(A1#)",
        "set E5 MIN",
        "set F5 =MIN(A1#)",
        "set E6 COUNT",
        "set F6 =COUNT(A1#)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Sequence: fg Fern + bg Mist
    select_rect(&mut app, &mut io, 1, 1, 3, 6);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);

    // Aggregate headers: bold + Cloud + Lagoon
    select_rect(&mut app, &mut io, 5, 1, 6, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg lagoon"],
    );

    // Aggregate values: bold + Cloud + Lagoon
    select_rect(&mut app, &mut io, 5, 2, 5, 6);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt fg cloud", "fmt bg lagoon"]);
    select_rect(&mut app, &mut io, 6, 2, 6, 6);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg lagoon"],
    );

    select_rect(&mut app, &mut io, 1, 1, 1, 1);
    write_scene(&app, path)
}

fn capture_randarray_lab(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        "set A1 =RANDARRAY(6,3,0,100,FALSE)",
        "set E1 Stat",
        "set F1 Value",
        "set E2 SUM",
        "set F2 =SUM(A1#)",
        "set E3 AVG",
        "set F3 =AVERAGE(A1#)",
        "set E4 MAX",
        "set F4 =MAX(A1#)",
        "set E5 MIN",
        "set F5 =MIN(A1#)",
        "set E6 COUNT",
        "set F6 =COUNT(A1#)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Array: fg Lavender + bg Cloud + decimals 2
    select_rect(&mut app, &mut io, 1, 1, 3, 6);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt fg lavender", "fmt bg cloud", "fmt decimals 2"],
    );

    // Stats headers: bold + Cloud + Teal
    select_rect(&mut app, &mut io, 5, 1, 6, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Stats values: bold + Cloud + Teal + decimals 2
    select_rect(&mut app, &mut io, 5, 2, 5, 6);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt fg cloud", "fmt bg teal"]);
    select_rect(&mut app, &mut io, 6, 2, 6, 6);
    apply_commands(
        &mut app,
        &mut io,
        &[
            "fmt bold on",
            "fmt fg cloud",
            "fmt bg teal",
            "fmt decimals 2",
        ],
    );

    select_rect(&mut app, &mut io, 1, 1, 1, 1);
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
        "set J2 C1=MAP(LAMBDA SEQ 1x3)",
        "set J3 G1=MAP(LAMBDA SEQ 2x1)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Tile 1: fg Fern + bg Mist
    select_rect(&mut app, &mut io, 3, 1, 5, 3);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);

    // Tile 2: fg Peach + bg Seafoam
    select_rect(&mut app, &mut io, 7, 1, 7, 6);
    apply_commands(&mut app, &mut io, &["fmt fg peach", "fmt bg seafoam"]);

    // Formula notes
    select_rect(&mut app, &mut io, 10, 1, 10, 3);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );

    select_rect(&mut app, &mut io, 3, 1, 3, 1);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Advanced Functions (scenes 15-17)
// ---------------------------------------------------------------------------

fn capture_let_lambda(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
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
        "set A4 Venture",
        "set B4 200000",
        "set C4 36",
        "set D4 =ROUND(PMT(BASE_RATE/12,C4,B4),2)",
        "set E4 =ROUND(LET(p,D4,m,C4,p*m*RISK_ADJ),2)",
        "set G1 Lambda",
        "set G2 =LET(scale,LAMBDA(x,x*RISK_ADJ),scale(D2))",
        "set G3 =LET(scale,LAMBDA(x,x*RISK_ADJ),scale(D3))",
        "set G4 =LET(scale,LAMBDA(x,x*RISK_ADJ),scale(D4))",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Headers: bold + Cloud + Teal
    select_rect(&mut app, &mut io, 1, 1, 5, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Results: fg Peach + decimals 2
    select_rect(&mut app, &mut io, 4, 2, 5, 4);
    apply_commands(&mut app, &mut io, &["fmt decimals 2", "fmt fg peach"]);

    // Lambda column: bold + Lavender + Olive
    select_rect(&mut app, &mut io, 7, 1, 7, 4);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg lavender", "fmt bg olive"],
    );

    select_rect(&mut app, &mut io, 7, 2, 7, 2);
    write_scene(&app, path)
}

fn capture_indirect_r1c1_offset(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        "set F4 D3 sum R1C1 range",
        "set F5 D4 OFFSET range",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Results: bold + Cloud + Moss
    select_rect(&mut app, &mut io, 4, 1, 4, 4);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg moss"],
    );

    // Notes: fg Sky
    select_rect(&mut app, &mut io, 6, 1, 6, 5);
    apply_commands(&mut app, &mut io, &["fmt fg sky"]);

    select_rect(&mut app, &mut io, 4, 3, 4, 3);
    write_scene(&app, path)
}

fn capture_lookup_model(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let commands = [
        // Product table (2-column: ID→Price)
        "set A1 ID",
        "set B1 Price",
        "set A2 100",
        "set B2 9.99",
        "set A3 200",
        "set B3 24.50",
        "set A4 300",
        "set B4 39.99",
        "set A5 400",
        "set B5 15.00",
        "set A6 500",
        "set B6 55.00",
        // Order lookup region
        "set D1 Order",
        "set E1 LookupID",
        "set F1 Price",
        "set D2 1",
        "set E2 200",
        "set F2 =LOOKUP(E2,A2:B6)",
        "set D3 2",
        "set E3 400",
        "set F3 =LOOKUP(E3,A2:B6)",
        "set D4 3",
        "set E4 100",
        "set F4 =LOOKUP(E4,A2:B6)",
        "set D5 4",
        "set E5 500",
        "set F5 =LOOKUP(E5,A2:B6)",
    ];
    apply_commands(&mut app, &mut io, &commands);

    // Product headers
    select_rect(&mut app, &mut io, 1, 1, 2, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Product IDs: fg Fern
    select_rect(&mut app, &mut io, 1, 2, 1, 6);
    apply_commands(&mut app, &mut io, &["fmt fg fern"]);

    // Product prices: fg Peach + decimals 2
    select_rect(&mut app, &mut io, 2, 2, 2, 6);
    apply_commands(&mut app, &mut io, &["fmt fg peach", "fmt decimals 2"]);

    // Order headers
    select_rect(&mut app, &mut io, 4, 1, 6, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg teal"],
    );

    // Lookup results: bold + Lavender + Mist + decimals 2
    select_rect(&mut app, &mut io, 6, 2, 6, 5);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg lavender", "fmt bg mist", "fmt decimals 2"],
    );

    select_rect(&mut app, &mut io, 6, 2, 6, 2);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Logic & Text (scenes 18-19)
// ---------------------------------------------------------------------------

fn capture_student_gradebook(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let students = [
        ("Alice", 95),
        ("Bob", 88),
        ("Carol", 72),
        ("Dave", 91),
        ("Eve", 65),
        ("Frank", 83),
        ("Grace", 78),
    ];

    let mut commands: Vec<String> = Vec::new();
    commands.push("set A1 Student".to_string());
    commands.push("set B1 Score".to_string());
    commands.push("set C1 Grade".to_string());
    commands.push("set D1 Honor".to_string());
    commands.push("set E1 Result".to_string());

    for (i, (name, score)) in students.iter().enumerate() {
        let r = i + 2;
        commands.push(format!("set A{} {}", r, name));
        commands.push(format!("set B{} {}", r, score));
        commands.push(format!(
            "set C{} =IF(B{}>=90,\"A\",IF(B{}>=80,\"B\",IF(B{}>=70,\"C\",\"F\")))",
            r, r, r, r
        ));
        commands.push(format!(
            "set D{} =IF(AND(B{}>=85,B{}<=100),\"Yes\",\"No\")",
            r, r, r
        ));
        commands.push(format!(
            "set E{} =IF(OR(B{}>=95,B{}<=60),\"Review\",\"OK\")",
            r, r, r
        ));
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    apply_commands(&mut app, &mut io, &cmd_refs);

    // Headers: bold + Cloud + Moss
    select_rect(&mut app, &mut io, 1, 1, 5, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg moss"],
    );

    // Grade column: bold + Lavender
    select_rect(&mut app, &mut io, 3, 2, 3, 8);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt fg lavender"]);

    // Honor + Result: fg Rose
    select_rect(&mut app, &mut io, 4, 2, 5, 8);
    apply_commands(&mut app, &mut io, &["fmt fg rose"]);

    select_rect(&mut app, &mut io, 3, 2, 3, 2);
    write_scene(&app, path)
}

fn capture_text_functions(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();

    let names = [
        ("Alice", "Smith"),
        ("Bob", "Jones"),
        ("Carol", "Williams"),
        ("Dave", "Brown"),
        ("Eve", "Johnson"),
    ];

    let mut commands: Vec<String> = Vec::new();
    commands.push("set A1 First".to_string());
    commands.push("set B1 Last".to_string());
    commands.push("set C1 Full".to_string());
    commands.push("set D1 Len".to_string());
    commands.push("set E1 Check".to_string());

    for (i, (first, last)) in names.iter().enumerate() {
        let r = i + 2;
        commands.push(format!("set A{} {}", r, first));
        commands.push(format!("set B{} {}", r, last));
        commands.push(format!("set C{} =CONCAT(A{},\" \",B{})", r, r, r));
        commands.push(format!("set D{} =LEN(C{})", r, r));
        commands.push(format!(
            "set E{} =IF(D{}>=10,\"Long\",\"OK\")",
            r, r
        ));
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    apply_commands(&mut app, &mut io, &cmd_refs);

    // Headers: bold + Cloud + Slate
    select_rect(&mut app, &mut io, 1, 1, 5, 1);
    apply_commands(
        &mut app,
        &mut io,
        &["fmt bold on", "fmt fg cloud", "fmt bg slate"],
    );

    // Text columns: fg Sky + bg Sand
    select_rect(&mut app, &mut io, 1, 2, 3, 6);
    apply_commands(&mut app, &mut io, &["fmt fg sky", "fmt bg sand"]);

    // Results: bold + fg Fern
    select_rect(&mut app, &mut io, 4, 2, 5, 6);
    apply_commands(&mut app, &mut io, &["fmt bold on", "fmt fg fern"]);

    select_rect(&mut app, &mut io, 3, 3, 3, 3);
    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Clipboard (scene 20)
// ---------------------------------------------------------------------------

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

    // Format source cells: fg Fern + bg Mist
    select_rect(&mut app, &mut io, 1, 1, 2, 2);
    apply_commands(&mut app, &mut io, &["fmt fg fern", "fmt bg mist"]);

    // Select source range and copy
    select_rect(&mut app, &mut io, 1, 1, 2, 2);
    app.apply(Action::CopySelection, &mut io);

    let clipboard_text = app
        .last_copy_text()
        .map(ToString::to_string)
        .unwrap_or_else(|| "Product\tUnits\nDNA\t30".to_string());

    // Navigate to destination and begin paste special
    move_to_cell(&mut app, &mut io, 4, 4);
    app.apply(
        Action::BeginPasteFromClipboard(clipboard_text),
        &mut io,
    );
    app.apply(Action::InputChar('2'), &mut io);

    write_scene(&app, path)
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

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
