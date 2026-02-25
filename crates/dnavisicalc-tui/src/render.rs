use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode, SpillRole};

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
            let style = if cell.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match cell.spill_role {
                    SpillRole::Anchor => Style::default()
                        .fg(Color::White)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    SpillRole::Member => Style::default().fg(Color::Cyan),
                    SpillRole::None => Style::default(),
                }
            };
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
    };
    let input_text = match app.mode() {
        AppMode::Edit => format!("Edit {}", app.edit_buffer()),
        AppMode::Command => format!(":{}", app.command_buffer()),
        AppMode::Navigate => app.formula_or_input_for_selected(),
    };

    let spill_text = app
        .spill_info_for_selected()
        .unwrap_or_else(|| "Spill -".to_string());
    frame.render_widget(
        Paragraph::new(format!(
            "{} | Cell {} | Value {} | {}",
            mode_text,
            app.selected_cell(),
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
        let popup = centered_rect(80, 75, frame.area());
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
            "Nav: arrows/hjkl move | Enter/e edit | : command | r recalc | ?/F1 help | q quit"
        }
        AppMode::Edit => "Edit: type value/formula | Backspace delete | Enter apply | Esc cancel",
        AppMode::Command => {
            "Command: type command | Enter run | Esc cancel | help command or ?/F1 for full help"
        }
    }
}

fn help_text() -> String {
    let function_list = dnavisicalc_core::SUPPORTED_FUNCTIONS.join(", ");
    format!(
        "DNA VisiCalc\n\
\n\
Navigation keys\n\
- Arrows or h/j/k/l: move selection\n\
- Enter or e: edit selected cell\n\
- : enter command mode\n\
- r: recalculate\n\
- q: quit\n\
- ? or F1: toggle this help\n\
\n\
Command mode\n\
- w <path> or write <path>: save workbook\n\
- o <path> or open <path>: open workbook\n\
- w with no path: write to last saved path\n\
- set <A1> <value|formula>: assign a cell\n\
- name <NAME> <value|formula>: assign workbook name\n\
- name clear <NAME>: remove workbook name\n\
- mode auto|manual: recalc mode\n\
- r or recalc: recalculate now\n\
- q or quit: quit\n\
\n\
Supported functions\n\
- {function_list}\n\
\n\
Notes\n\
- File/Save status is shown in the top bar.\n\
- Status messages persist while navigating.\n\
- Spill child cells are read-only; edit the anchor cell.\n\
\n\
Press ? or F1 to close help."
    )
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
