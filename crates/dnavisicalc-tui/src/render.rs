use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AppMode};

pub fn render_app(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let (grid_width, grid_height) = compute_grid_dimensions(chunks[0]);
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
                Style::default()
            };
            spans.push(Span::styled(format!("{text:<8}"), style));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
    }

    let grid_block =
        Paragraph::new(lines).block(Block::default().title("DNA VisiCalc").borders(Borders::ALL));
    frame.render_widget(grid_block, chunks[0]);

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

    frame.render_widget(
        Paragraph::new(format!(
            "{} | Cell {} | Value {}",
            mode_text,
            app.selected_cell(),
            app.evaluate_display_for_selected()
        ))
        .block(Block::default().title("Context").borders(Borders::ALL)),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(format!("{}\n{}", input_text, app.status())).block(
            Block::default()
                .title("Input / Status")
                .borders(Borders::ALL),
        ),
        chunks[2],
    );
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
}
