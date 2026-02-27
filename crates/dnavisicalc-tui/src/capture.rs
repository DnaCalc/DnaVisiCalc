use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

use crate::{App, render_app};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSize {
    pub width: u16,
    pub height: u16,
}

impl CaptureSize {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

impl Default for CaptureSize {
    fn default() -> Self {
        Self {
            width: 140,
            height: 40,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureCursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSpan {
    pub text: String,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRow {
    pub y: u16,
    pub spans: Vec<CaptureSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrame {
    pub width: u16,
    pub height: u16,
    pub cursor: Option<CaptureCursor>,
    pub rows: Vec<CaptureRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineFrame {
    pub label: Option<String>,
    pub mode: Option<String>,
    pub key: Option<String>,
    pub action: Option<String>,
    pub frame: CaptureFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureTimeline {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<TimelineFrame>,
}

impl CaptureTimeline {
    pub fn new(size: CaptureSize) -> Self {
        Self {
            width: size.width,
            height: size.height,
            frames: Vec::new(),
        }
    }

    pub fn push_frame(&mut self, entry: TimelineFrame) {
        self.frames.push(entry);
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let timeline = serde_json::from_str::<Self>(&text)?;
        Ok(timeline)
    }
}

pub fn capture_app_frame(app: &App, size: CaptureSize) -> Result<CaptureFrame> {
    if size.width == 0 || size.height == 0 {
        return Err(anyhow!("capture size must be at least 1x1"));
    }

    let backend = TestBackend::new(size.width, size.height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_app(frame, app))?;
    let cursor_position = terminal.get_cursor_position().ok();
    let buffer = terminal.backend().buffer();

    let width = buffer.area().width as usize;
    let height = buffer.area().height as usize;

    let cursor = cursor_position.and_then(|pos| {
        if pos.x < buffer.area().width && pos.y < buffer.area().height {
            Some(CaptureCursor { x: pos.x, y: pos.y })
        } else {
            None
        }
    });

    let content = buffer.content();
    let mut rows: Vec<CaptureRow> = Vec::with_capacity(height);

    for y in 0..height {
        let row_start = y * width;
        let mut spans: Vec<CaptureSpan> = Vec::new();
        for x in 0..width {
            let cell = &content[row_start + x];
            let fg = color_to_hex(cell.fg);
            let bg = color_to_hex(cell.bg);
            let bold = cell.modifier.contains(Modifier::BOLD);
            let italic = cell.modifier.contains(Modifier::ITALIC);
            let symbol = cell.symbol();

            if let Some(last) = spans.last_mut()
                && last.fg == fg
                && last.bg == bg
                && last.bold == bold
                && last.italic == italic
            {
                last.text.push_str(symbol);
                continue;
            }

            spans.push(CaptureSpan {
                text: symbol.to_string(),
                fg,
                bg,
                bold,
                italic,
            });
        }

        rows.push(CaptureRow { y: y as u16, spans });
    }

    Ok(CaptureFrame {
        width: width as u16,
        height: height as u16,
        cursor,
        rows,
    })
}

pub fn frame_to_text(frame: &CaptureFrame, trim_trailing: bool) -> String {
    frame
        .rows
        .iter()
        .map(|row| {
            let mut line = String::new();
            for span in &row.spans {
                line.push_str(&span.text);
            }
            if trim_trailing {
                line.trim_end().to_string()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn write_frame_text(
    frame: &CaptureFrame,
    path: impl AsRef<Path>,
    trim_trailing: bool,
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, frame_to_text(frame, trim_trailing))?;
    Ok(())
}

pub fn write_frame_json(frame: &CaptureFrame, path: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(frame)?;
    fs::write(path, serialized)?;
    Ok(())
}

pub fn write_frame_svg(frame: &CaptureFrame, path: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    const CELL_W: usize = 10;
    const CELL_H: usize = 18;
    let canvas_w = frame.width as usize * CELL_W;
    let canvas_h = frame.height as usize * CELL_H;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{canvas_w}\" height=\"{canvas_h}\" viewBox=\"0 0 {canvas_w} {canvas_h}\">\n"
    ));
    svg.push_str("<rect x=\"0\" y=\"0\" width=\"100%\" height=\"100%\" fill=\"#000000\"/>\n");

    for row in &frame.rows {
        let y_index = row.y as usize;
        let y_px = y_index * CELL_H;
        let baseline = y_px + CELL_H - 4;
        let mut x_chars = 0usize;

        for span in &row.spans {
            let char_count = span.text.chars().count();
            let span_width_px = char_count * CELL_W;
            let x_px = x_chars * CELL_W;

            if let Some(bg) = span.bg.as_deref() {
                svg.push_str(&format!(
                    "<rect x=\"{x_px}\" y=\"{y_px}\" width=\"{span_width_px}\" height=\"{CELL_H}\" fill=\"{bg}\"/>\n"
                ));
            }

            let fg = span.fg.as_deref().unwrap_or("#E0E0E0");
            let font_weight = if span.bold { "700" } else { "400" };
            let font_style = if span.italic { "italic" } else { "normal" };
            svg.push_str(&format!(
                "<text x=\"{x_px}\" y=\"{baseline}\" fill=\"{fg}\" font-family=\"Consolas, Menlo, Monaco, monospace\" font-size=\"14\" font-style=\"{font_style}\" font-weight=\"{font_weight}\" xml:space=\"preserve\">{}</text>\n",
                xml_escape(&span.text)
            ));

            x_chars += char_count;
        }
    }

    if let Some(cursor) = frame.cursor {
        let x_px = cursor.x as usize * CELL_W;
        let y_px = cursor.y as usize * CELL_H;
        svg.push_str(&format!(
            "<rect x=\"{x_px}\" y=\"{y_px}\" width=\"{CELL_W}\" height=\"{CELL_H}\" fill=\"none\" stroke=\"#00FFFF\" stroke-width=\"1\"/>\n"
        ));
    }

    svg.push_str("</svg>\n");
    fs::write(path, svg)?;
    Ok(())
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

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
