use std::env;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dnavisicalc_tui::{CaptureFrame, CaptureSpan, CaptureTimeline};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

struct ViewerState {
    frame_index: usize,
    playing: bool,
    speed_fps: f64,
    accumulator: f64,
    last_tick: Instant,
}

impl ViewerState {
    fn new() -> Self {
        Self {
            frame_index: 0,
            playing: false,
            speed_fps: 2.0,
            accumulator: 0.0,
            last_tick: Instant::now(),
        }
    }

    fn step_by(&mut self, delta: isize, frame_count: usize) {
        if frame_count == 0 {
            self.frame_index = 0;
            return;
        }
        let max_index = frame_count as isize - 1;
        let next = (self.frame_index as isize + delta).clamp(0, max_index);
        self.frame_index = next as usize;
    }

    fn clamp_to_bounds(&mut self, frame_count: usize) {
        if frame_count == 0 {
            self.frame_index = 0;
            return;
        }
        if self.frame_index >= frame_count {
            self.frame_index = frame_count - 1;
        }
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        bail!("usage: capture_viewer <timeline.json>");
    };

    let timeline = CaptureTimeline::load_json(PathBuf::from(path))?;
    if timeline.frames.is_empty() {
        bail!("timeline has no frames");
    }

    run_viewer(timeline)
}

fn run_viewer(timeline: CaptureTimeline) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = ViewerState::new();
    let frame_count = timeline.frames.len();
    let run_result = loop_viewer(&mut terminal, &timeline, &mut state, frame_count);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}

fn loop_viewer(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    timeline: &CaptureTimeline,
    state: &mut ViewerState,
    frame_count: usize,
) -> Result<()> {
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_tick).as_secs_f64();
        state.last_tick = now;

        if state.playing {
            state.accumulator += elapsed;
            let step_interval = (1.0 / state.speed_fps).max(0.001);
            while state.accumulator >= step_interval {
                state.accumulator -= step_interval;
                if state.frame_index + 1 < frame_count {
                    state.frame_index += 1;
                } else {
                    state.playing = false;
                    state.accumulator = 0.0;
                    break;
                }
            }
        }
        state.clamp_to_bounds(frame_count);

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            let current = &timeline.frames[state.frame_index];
            let frame_label = current.label.as_deref().unwrap_or("-");
            let key = current.key.as_deref().unwrap_or("-");
            let action = current.action.as_deref().unwrap_or("-");
            let mode = current.mode.as_deref().unwrap_or("-");
            let status = if state.playing { "PLAY" } else { "PAUSE" };

            let header = format!(
                "Capture Viewer | {status} | frame {}/{} | {:.2} fps | label {frame_label}",
                state.frame_index + 1,
                frame_count,
                state.speed_fps
            );
            frame.render_widget(
                Paragraph::new(header)
                    .block(Block::default().title("Timeline").borders(Borders::ALL)),
                chunks[0],
            );

            render_capture_frame(frame, chunks[1], &current.frame);

            let footer = format!(
                "mode {mode} | key {key} | action {action} | Space play/pause, <-/-> +/-1, [/ ] +/-15, +/- speed, Home/End, q quit"
            );
            frame.render_widget(
                Paragraph::new(footer)
                    .wrap(Wrap { trim: false })
                    .block(Block::default().title("Controls").borders(Borders::ALL)),
                chunks[2],
            );
        })?;

        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char(' ') => state.playing = !state.playing,
                        KeyCode::Right => {
                            state.playing = false;
                            state.step_by(1, frame_count);
                        }
                        KeyCode::Left => {
                            state.playing = false;
                            state.step_by(-1, frame_count);
                        }
                        KeyCode::Char(']') => {
                            state.playing = false;
                            state.step_by(15, frame_count);
                        }
                        KeyCode::Char('[') => {
                            state.playing = false;
                            state.step_by(-15, frame_count);
                        }
                        KeyCode::Home => {
                            state.playing = false;
                            state.frame_index = 0;
                        }
                        KeyCode::End => {
                            state.playing = false;
                            state.frame_index = frame_count.saturating_sub(1);
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            state.speed_fps = (state.speed_fps * 2.0).min(60.0);
                        }
                        KeyCode::Char('-') => {
                            state.speed_fps = (state.speed_fps / 2.0).max(0.25);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_capture_frame(frame: &mut ratatui::Frame, area: Rect, snapshot: &CaptureFrame) {
    let block = Block::default().title("Frame Buffer").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let content_area = Rect {
        x: inner.x.saturating_add(1),
        y: inner.y.saturating_add(1),
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(2),
    };
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for row in snapshot.rows.iter().take(content_area.height as usize) {
        let mut spans: Vec<Span> = Vec::new();
        for span in &row.spans {
            spans.push(Span::styled(
                span.text.clone(),
                style_from_capture_span(span),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        content_area,
    );

    if let Some(cursor) = snapshot.cursor
        && cursor.x < content_area.width
        && cursor.y < content_area.height
    {
        frame.set_cursor_position((content_area.x + cursor.x, content_area.y + cursor.y));
    }
}

fn style_from_capture_span(span: &CaptureSpan) -> Style {
    let mut style = Style::default();
    if let Some(fg) = span.fg.as_deref().and_then(parse_hex_color) {
        style = style.fg(fg);
    }
    if let Some(bg) = span.bg.as_deref().and_then(parse_hex_color) {
        style = style.bg(bg);
    }
    if span.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if span.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    style
}

fn parse_hex_color(text: &str) -> Option<Color> {
    let value = text.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
