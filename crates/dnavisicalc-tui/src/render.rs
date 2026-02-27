use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode, ChartData, ControlKind, PasteMode, SpillRole};
use dnavisicalc_engine::PaletteColor;

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
        dnavisicalc_engine::RecalcMode::Automatic => "auto",
        dnavisicalc_engine::RecalcMode::Manual => "manual",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "File: {file_name} | Save: {} | Recalc: {recalc_mode} | F3 Controls | F9 Recalc | ?/F1 Help",
            app.save_state_label()
        ))
        .block(Block::default().title("Workbook").borders(Borders::ALL)),
        chunks[0],
    );

    let chart_data = app.chart_data();
    let has_controls = !app.controls().is_empty();
    if chart_data.is_some() || has_controls {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(67), Constraint::Percentage(33)])
            .split(chunks[1]);
        render_grid(frame, app, h_chunks[0]);
        render_right_panel(frame, app, chart_data.as_ref(), h_chunks[1]);
    } else {
        render_grid(frame, app, chunks[1]);
    }

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
    let status_line = if app.mode() == AppMode::Command {
        let cmd_hint = app.command_hint();
        if cmd_hint.is_empty() {
            format!("Status: {}", app.status())
        } else {
            format!("Status: {} | >> {}", app.status(), cmd_hint)
        }
    } else {
        format!("Status: {}", app.status())
    };
    frame.render_widget(
        Paragraph::new(format!("{input_text}\n{status_line}\n{hints}"))
            .block(
                Block::default()
                    .title("Input / Status / Help")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[3],
    );

    if matches!(app.mode(), AppMode::Edit | AppMode::Command) {
        let cursor_x = chunks[3].x + 1 + input_text.len() as u16;
        let cursor_y = chunks[3].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    if app.help_visible() {
        let popup = centered_rect(92, 92, frame.area());
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(help_lines())
                .block(Block::default().title("Help").borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            popup,
        );
    }
}

fn render_grid(frame: &mut Frame, app: &App, area: Rect) {
    let (grid_width, grid_height) = compute_grid_dimensions(area);
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
    frame.render_widget(grid_block, area);
}

fn render_right_panel(frame: &mut Frame, app: &App, chart_data: Option<&ChartData>, area: Rect) {
    let has_chart = chart_data.is_some();
    let has_controls = !app.controls().is_empty();

    match (has_chart, has_controls) {
        (true, true) => {
            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            render_bar_chart(frame, chart_data.unwrap(), v_chunks[0]);
            render_controls_panel(frame, app, v_chunks[1]);
        }
        (true, false) => {
            render_bar_chart(frame, chart_data.unwrap(), area);
        }
        (false, true) => {
            render_controls_panel(frame, app, area);
        }
        (false, false) => {}
    }
}

fn render_controls_panel(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.controls_focused() {
        "Controls [F3]"
    } else {
        "Controls"
    };
    let border_style = if app.controls_focused() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let controls = app.controls();
    let focused_idx = app.controls_focus();
    let is_focused = app.controls_focused();

    let mut lines: Vec<Line> = Vec::new();
    for (i, ctrl) in controls.iter().enumerate() {
        let is_active = is_focused && i == focused_idx;
        let prefix = if is_active { "\u{25b8} " } else { "  " };

        let line = match ctrl.kind {
            ControlKind::Slider => {
                let bar_width = (inner.width as usize).saturating_sub(prefix.len() + 10 + 5);
                let bar_width = bar_width.max(2);
                let filled = ((ctrl.value / 100.0) * bar_width as f64).round() as usize;
                let empty = bar_width.saturating_sub(filled);
                let bar = format!(
                    "{}[{}{}] {:>3}",
                    prefix,
                    "\u{2588}".repeat(filled),
                    "\u{2591}".repeat(empty),
                    ctrl.value as i64
                );
                let name_display = truncate_cell_text(&ctrl.name, 8);
                let full = format!("{name_display:<8} {bar}");
                if is_active {
                    Line::from(Span::styled(
                        full,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(full)
                }
            }
            ControlKind::Checkbox => {
                let check = if ctrl.value != 0.0 { "x" } else { " " };
                let text = format!("{prefix}[{check}] {}", ctrl.name);
                if is_active {
                    Line::from(Span::styled(
                        text,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(text)
                }
            }
            ControlKind::Button => {
                let text = format!(
                    "{prefix}\u{00ab}{}\u{00bb} {}",
                    ctrl.name, ctrl.value as i64
                );
                if is_active {
                    Line::from(Span::styled(
                        text,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(text)
                }
            }
        };
        lines.push(line);
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_bar_chart(frame: &mut Frame, chart_data: &ChartData, area: Rect) {
    let title = format!("Bar Chart ({})", chart_data.range_label);
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let bar_colors = [
        Color::Rgb(125, 157, 116), // Fern
        Color::Rgb(165, 208, 225), // Sky
        Color::Rgb(235, 188, 156), // Peach
        Color::Rgb(187, 176, 214), // Lavender
        Color::Rgb(224, 207, 176), // Sand
        Color::Rgb(104, 166, 158), // Lagoon
    ];

    let max_value = chart_data
        .series
        .iter()
        .flat_map(|s| s.values.iter())
        .cloned()
        .fold(0.0_f64, f64::max);

    let num_labels = chart_data.labels.len();
    let num_series = chart_data.series.len().max(1);
    let bars_per_label = num_series;
    let total_bars = num_labels * bars_per_label;
    let available_height = inner.height as usize;

    let label_width = 6usize;
    // space for "| " prefix + label + " " + bar + " " + value
    let bar_area_width = (inner.width as usize).saturating_sub(label_width + 1 + 8);

    let mut lines: Vec<Line> = Vec::new();
    let mut bars_shown = 0usize;

    for (label_idx, label) in chart_data.labels.iter().enumerate() {
        for (series_idx, series) in chart_data.series.iter().enumerate() {
            if bars_shown >= available_height.saturating_sub(1) && bars_shown < total_bars {
                let remaining = total_bars - bars_shown;
                lines.push(Line::from(Span::styled(
                    format!("  ... +{remaining} more"),
                    Style::default().fg(Color::DarkGray),
                )));
                bars_shown = total_bars;
                break;
            }

            let value = series.values.get(label_idx).copied().unwrap_or(0.0);
            let bar_len = if max_value > 0.0 {
                ((value / max_value) * bar_area_width as f64).round() as usize
            } else {
                0
            };

            let display_label = if num_series > 1 && series_idx == 0 {
                truncate_cell_text(label, label_width)
            } else if num_series > 1 {
                " ".repeat(label_width.min(label.len().max(1)))
            } else {
                truncate_cell_text(label, label_width)
            };

            let color = bar_colors[series_idx % bar_colors.len()];
            let bar_str = "\u{2588}".repeat(bar_len);
            let value_str = format!(" {value}");

            let mut spans = vec![
                Span::raw(format!("{display_label:<label_width$} ")),
                Span::styled(bar_str, Style::default().fg(color)),
                Span::raw(value_str),
            ];

            if num_series > 1 {
                spans.push(Span::styled(
                    format!(" [{}]", series.name),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(spans));
            bars_shown += 1;
        }
        if bars_shown >= total_bars {
            break;
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
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
            "Nav: arrows/hjkl move | Shift select | Del clear | type to edit | Enter/e/F2 edit | Ctrl+C/V copy/paste | F3 controls | F9 recalc | : cmd | ?/F1 help"
        }
        AppMode::Edit => "Edit: type value/formula | Backspace delete | Enter apply | Esc discard",
        AppMode::Command => "Cmd: Enter run | Esc cancel",
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

fn help_lines() -> Vec<Line<'static>> {
    let function_list = dnavisicalc_engine::SUPPORTED_FUNCTIONS.join(", ");
    let mut lines: Vec<Line> = Vec::new();

    for s in [
        "DNA VisiCalc",
        "",
        "Navigation keys",
        "- Arrows or h/j/k/l: move selection",
        "- Shift+Arrows or Shift+H/J/K/L: extend selection",
        "- Ctrl+C / Ctrl+V: copy / paste (Paste Special)",
        "- Delete: clear selected cell/range contents",
        "- Enter, e, or F2: edit selected cell",
        "- F3: focus controls panel",
        "- F9: force recalculation now",
        "- : enter command mode | ? or F1: toggle help",
        "",
    ] {
        lines.push(Line::from(s));
    }

    lines.push(Line::from(format!("Supported functions: {function_list}")));
    lines.push(Line::from(""));

    // Two-column layout: Paste Special (left) | Command mode (right)
    let col_w: usize = 44;
    let left_col: Vec<&str> = vec![
        "Paste Special (after Ctrl+V)",
        "  1 All: formulas/values + formatting",
        "  2 Formulas: formulas only",
        "  3 Values: values only",
        "  4 Values+KeepDestFmt: keep dest fmt",
        "  5 Formatting: formatting only",
        "  Enter apply, Esc cancel, Tab cycle",
        "",
        "Controls (F3 to focus panel)",
        "  ↑↓ navigate, ←→ adjust slider",
        "  Shift+←→ adjust by 10",
        "  Space toggle checkbox / press button",
        "  Esc back to grid",
    ];
    let right_col: Vec<&str> = vec![
        "Command mode (:)",
        "  w [path]: save | o <path>: open",
        "  set <A1> <value|formula>: assign cell",
        "  name <NAME> <expr>: assign name",
        "  name clear <NAME>: remove name",
        "  fmt decimals|bold|italic|fg|bg|clear",
        "  chart: toggle bar chart from selection",
        "  ctrl add slider|checkbox|button <NAME>",
        "  ctrl remove <NAME> | ctrl list",
        "  mode auto|manual: recalc mode",
        "  r/recalc: recalculate | q: quit",
        "",
        "Notes",
    ];

    let max_rows = left_col.len().max(right_col.len());
    for i in 0..max_rows {
        let left = left_col.get(i).copied().unwrap_or("");
        let right = right_col.get(i).copied().unwrap_or("");
        lines.push(Line::from(format!("{:<col_w$}{}", left, right)));
    }
    lines.push(Line::from(""));

    let palette = [
        ("MIST", PaletteColor::Mist),
        ("SAGE", PaletteColor::Sage),
        ("FERN", PaletteColor::Fern),
        ("MOSS", PaletteColor::Moss),
        ("OLIVE", PaletteColor::Olive),
        ("SEAFOAM", PaletteColor::Seafoam),
        ("LAGOON", PaletteColor::Lagoon),
        ("TEAL", PaletteColor::Teal),
        ("SKY", PaletteColor::Sky),
        ("CLOUD", PaletteColor::Cloud),
        ("SAND", PaletteColor::Sand),
        ("CLAY", PaletteColor::Clay),
        ("PEACH", PaletteColor::Peach),
        ("ROSE", PaletteColor::Rose),
        ("LAVENDER", PaletteColor::Lavender),
        ("SLATE", PaletteColor::Slate),
    ];
    // Two rows of 8 colors each
    for row_start in [0usize, 8] {
        let mut color_spans: Vec<Span> = Vec::new();
        color_spans.push(Span::raw("  "));
        for i in row_start..palette.len().min(row_start + 8) {
            let (name, pc) = palette[i];
            if i > row_start {
                color_spans.push(Span::raw("  "));
            }
            color_spans.push(Span::styled(
                format!("\u{2588}\u{2588} {name}"),
                Style::default().fg(palette_color_to_tui(pc)),
            ));
        }
        lines.push(Line::from(color_spans));
    }

    lines.push(Line::from(""));
    for s in [
        "- Spill child cells are read-only; edit the anchor cell.",
        "Press ? or F1 or Esc to close help.",
    ] {
        lines.push(Line::from(s));
    }

    lines
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
        assert!(text.contains("?/F1 Help"));
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
        assert!(text.contains("name <NAME> <expr>"));
    }

    #[test]
    fn renders_controls_panel_when_controls_exist() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("create terminal");

        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();

        // Add a slider control
        app.apply(Action::StartCommand, &mut io);
        for ch in "ctrl add slider RATE".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
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

        assert!(text.contains("Controls"));
        assert!(text.contains("RATE"));
    }

    #[test]
    fn renders_chart_panel_when_active() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("create terminal");

        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();

        app.apply(Action::StartEdit, &mut io);
        app.apply(Action::InputChar('5'), &mut io);
        app.apply(Action::Submit, &mut io);

        app.apply(Action::ToggleChart, &mut io);

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("draw app");

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(text.contains("Bar Chart"));
        assert!(text.contains("DNA VisiCalc"));
    }
}
