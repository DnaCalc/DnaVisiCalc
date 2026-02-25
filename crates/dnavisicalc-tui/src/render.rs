use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode, PasteMode, SpillRole};
use dnavisicalc_core::PaletteColor;

pub fn render_app(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(4),
        ])
        .split(frame.area());

    let file_name = app.current_path().unwrap_or("[new workbook]");
    let recalc_mode = match app.engine().recalc_mode() {
        dnavisicalc_core::RecalcMode::Automatic => "auto",
        dnavisicalc_core::RecalcMode::Manual => "manual",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "File: {file_name} | Save: {} | Recalc: {recalc_mode} | Help: ?/F1",
            app.save_state_label()
        ))
        .block(Block::default().title("Workbook").borders(Borders::ALL)),
        chunks[0],
    );

    let (grid_width, grid_height) = compute_grid_dimensions(chunks[1]);
    let grid = app.visible_grid(grid_width, grid_height);
    let mut lines: Vec<Line> = Vec::new();

    let mut header = String::from("     ");
    for col in &grid.headers {
        header.push_str(&format!("| {:^8} ", col));
    }
    lines.push(Line::from(header));
    lines.push(Line::from("-".repeat(5 + (grid_width as usize * 11))));

    for row in &grid.rows {
        let mut spans = vec![Span::raw(format!("{:>4} ", row.row_label))];
        for cell in &row.cells {
            spans.push(Span::raw("| "));
            let text = truncate_cell_text(&cell.value, 8);
            let mut style = Style::default();
            if let Some(fg) = cell.format.fg {
                style = style.fg(palette_color_to_tui(fg));
            }
            if let Some(bg) = cell.format.bg {
                style = style.bg(palette_color_to_tui(bg));
            }
            if cell.is_text && cell.format.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if cell.is_text && cell.format.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            style = match cell.spill_role {
                SpillRole::Anchor => style
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                SpillRole::Member => style.fg(Color::Cyan),
                SpillRole::None => style,
            };
            if cell.in_selection {
                style = style.bg(Color::Rgb(63, 94, 83));
            }
            if cell.active {
                style = Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(191, 223, 188))
                    .add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(format!("{text:<8}"), style));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
    }

    let grid_block =
        Paragraph::new(lines).block(Block::default().title("DNA VisiCalc").borders(Borders::ALL));
    frame.render_widget(grid_block, chunks[1]);

    let mode_text = match app.mode() {
        AppMode::Navigate => "Mode: NAV",
        AppMode::Edit => "Mode: EDIT",
        AppMode::Command => "Mode: CMD",
        AppMode::PasteSpecial => "Mode: PST",
    };
    let input_text = match app.mode() {
        AppMode::Edit => format!("Edit {}", app.edit_buffer()),
        AppMode::Command => format!(":{}", app.command_buffer()),
        AppMode::Navigate => app.formula_or_input_for_selected(),
        AppMode::PasteSpecial => paste_special_text(app),
    };

    let spill_text = app
        .spill_info_for_selected()
        .unwrap_or_else(|| "Spill -".to_string());
    let sel = app.selected_range();
    let selection_text = if sel.start == sel.end {
        sel.start.to_string()
    } else {
        format!("{}:{}", sel.start, sel.end)
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{} | Cell {} | Sel {} | Value {} | {}",
            mode_text,
            app.selected_cell(),
            selection_text,
            app.evaluate_display_for_selected(),
            spill_text
        ))
        .block(Block::default().title("Context").borders(Borders::ALL)),
        chunks[2],
    );

    let hints = quick_help(app.mode());
    frame.render_widget(
        Paragraph::new(format!("{input_text}\nStatus: {}\n{}", app.status(), hints))
            .block(
                Block::default()
                    .title("Input / Status / Help")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[3],
    );

    if app.help_visible() {
        let popup = centered_rect(92, 92, frame.area());
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(help_text())
                .block(Block::default().title("Help").borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            popup,
        );
    }
}

pub fn compute_grid_dimensions(area: Rect) -> (u16, u16) {
    // Account for block borders and row-label prefix: "#### | <cell>...".
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);

    let grid_width = (inner_width.saturating_sub(5) / 11).max(1);
    let grid_height = inner_height.saturating_sub(2).max(1);

    (grid_width, grid_height)
}

fn truncate_cell_text(input: &str, width: usize) -> String {
    let mut out = String::new();
    for ch in input.chars().take(width) {
        out.push(ch);
    }
    out
}

fn quick_help(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Navigate => {
            "Nav: arrows/hjkl move | Shift+arrows/HJKL select | Del clear | Enter/e/F2 edit | Ctrl+C copy | Ctrl+V paste | : command | ?/F1 help"
        }
        AppMode::Edit => "Edit: type value/formula | Backspace delete | Enter apply | Esc cancel",
        AppMode::Command => {
            "Command: type command | Enter run | Esc cancel | help command or ?/F1 for full help"
        }
        AppMode::PasteSpecial => {
            "Paste: 1-5 choose mode | Tab/Arrows/J/K cycle | Enter apply | Esc cancel"
        }
    }
}

fn paste_special_text(app: &App) -> String {
    let selected_mode = app.paste_mode().unwrap_or(PasteMode::All);
    let options = PasteMode::ALL
        .iter()
        .enumerate()
        .map(|(idx, mode)| {
            if *mode == selected_mode {
                format!(">{}.{}", idx + 1, mode.label())
            } else {
                format!(" {}.{}", idx + 1, mode.label())
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!("Paste Special @ {} | {options}", app.selected_cell())
}

fn help_text() -> String {
    let function_list = dnavisicalc_core::SUPPORTED_FUNCTIONS.join(", ");
    format!(
        "DNA VisiCalc\n\
\n\
Navigation keys\n\
- Arrows or h/j/k/l: move selection\n\
- Shift+Arrows or Shift+H/J/K/L: extend multi-cell selection\n\
- Ctrl+C: copy selected range to system clipboard\n\
- Ctrl+V: paste from system clipboard (opens Paste Special picker)\n\
- Delete: clear selected cell/range contents\n\
- Enter, e, or F2: edit selected cell\n\
- : enter command mode\n\
- r: recalculate\n\
- q: quit\n\
- ? or F1: toggle this help\n\
\n\
Supported functions\n\
- {function_list}\n\
\n\
Paste Special (after Ctrl+V)\n\
- 1 All: formulas/values + formatting\n\
- 2 Formulas: formulas only\n\
- 3 Values: values only (source formatting ignored)\n\
- 4 Values+KeepDestFmt: values only, preserve destination formatting\n\
- 5 Formatting: formatting only\n\
- Enter apply, Esc cancel, Tab/Arrows/J/K cycle\n\
\n\
Command mode\n\
- w <path> or write <path>: save workbook\n\
- o <path> or open <path>: open workbook\n\
- w with no path: write to last saved path\n\
- set <A1> <value|formula>: assign a cell\n\
- name <NAME> <value|formula>: assign workbook name\n\
- name clear <NAME>: remove workbook name\n\
- fmt decimals <0..9|none>: number decimals on selection\n\
- fmt bold on|off: text bold on selection\n\
- fmt italic on|off: text italic on selection\n\
- fmt fg/bg <color|none>: foreground/background color on selection\n\
- fmt clear: reset formatting on selection\n\
- mode auto|manual: recalc mode\n\
- r or recalc: recalculate now\n\
- q or quit: quit\n\
\n\
Notes\n\
- File/Save status is shown in the top bar.\n\
- Status messages persist while navigating.\n\
- Spill child cells are read-only; edit the anchor cell.\n\
- Palette: MIST, SAGE, FERN, MOSS, OLIVE, SEAFOAM, LAGOON, TEAL,\n\
  SKY, CLOUD, SAND, CLAY, PEACH, ROSE, LAVENDER, SLATE.\n\
\n\
Press ? or F1 to close help."
    )
}

fn palette_color_to_tui(color: PaletteColor) -> Color {
    match color {
        PaletteColor::Mist => Color::Rgb(232, 240, 236),
        PaletteColor::Sage => Color::Rgb(188, 205, 181),
        PaletteColor::Fern => Color::Rgb(125, 157, 116),
        PaletteColor::Moss => Color::Rgb(95, 121, 87),
        PaletteColor::Olive => Color::Rgb(146, 158, 92),
        PaletteColor::Seafoam => Color::Rgb(170, 223, 211),
        PaletteColor::Lagoon => Color::Rgb(104, 166, 158),
        PaletteColor::Teal => Color::Rgb(74, 132, 126),
        PaletteColor::Sky => Color::Rgb(165, 208, 225),
        PaletteColor::Cloud => Color::Rgb(214, 225, 232),
        PaletteColor::Sand => Color::Rgb(224, 207, 176),
        PaletteColor::Clay => Color::Rgb(187, 154, 130),
        PaletteColor::Peach => Color::Rgb(235, 188, 156),
        PaletteColor::Rose => Color::Rgb(216, 162, 171),
        PaletteColor::Lavender => Color::Rgb(187, 176, 214),
        PaletteColor::Slate => Color::Rgb(117, 132, 150),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::{Action, App};
    use crate::io::MemoryWorkbookIo;

    use super::*;

    #[test]
    fn renders_grid_and_status() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("create terminal");

        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();

        app.apply(Action::StartEdit, &mut io);
        app.apply(Action::InputChar('4'), &mut io);
        app.apply(Action::InputChar('2'), &mut io);
        app.apply(Action::Submit, &mut io);

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("draw app");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(text.contains("DNA VisiCalc"));
        assert!(text.contains("42"));
        assert!(text.contains("Mode: NAV"));
        assert!(text.contains("Help: ?/F1"));
    }

    #[test]
    fn computes_minimum_grid_dimensions_for_tiny_area() {
        let (w, h) = compute_grid_dimensions(Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        });
        assert_eq!((w, h), (1, 1));
    }

    #[test]
    fn help_popup_includes_supported_function_list() {
        let backend = TestBackend::new(140, 40);
        let mut terminal = Terminal::new(backend).expect("create terminal");
        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();
        app.apply(Action::ToggleHelp, &mut io);

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("draw app");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(text.contains("Supported functions"));
        assert!(text.contains("LOOKUP"));
        assert!(text.contains("PMT"));
        assert!(text.contains("name <NAME> <value|formula>"));
    }
}
