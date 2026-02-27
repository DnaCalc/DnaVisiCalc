use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use dnavisicalc_tui::{
    App, CaptureSize, CaptureTimeline, CommandOutcome, MemoryWorkbookIo, TimelineFrame,
    action_from_key, capture_app_frame, write_frame_json, write_frame_svg, write_frame_text,
};

#[derive(Debug, Clone)]
enum ScriptOp {
    Capture(Option<String>),
    Key(String),
    Text(String),
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(script_path) = args.next() else {
        bail!("usage: capture_script <script-path> <output-dir> [width] [height]");
    };
    let Some(output_dir) = args.next() else {
        bail!("usage: capture_script <script-path> <output-dir> [width] [height]");
    };

    let default_size = CaptureSize::default();
    let width = args
        .next()
        .map(|value| value.parse::<u16>())
        .transpose()?
        .unwrap_or(default_size.width);
    let height = args
        .next()
        .map(|value| value.parse::<u16>())
        .transpose()?
        .unwrap_or(default_size.height);
    let size = CaptureSize::new(width.max(1), height.max(1));

    let script_text = fs::read_to_string(&script_path)?;
    let script = parse_script(&script_text)?;

    let output_dir = PathBuf::from(output_dir);
    let frame_dir = output_dir.join("frames");
    fs::create_dir_all(&frame_dir)?;

    let mut app = App::new();
    let mut io = MemoryWorkbookIo::new();
    let mut timeline = CaptureTimeline::new(size);

    record_frame(
        &app,
        &mut timeline,
        &frame_dir,
        Some("initial".to_string()),
        None,
        None,
    )?;

    let mut quit = false;
    for op in script {
        if quit {
            break;
        }
        match op {
            ScriptOp::Capture(label) => {
                record_frame(&app, &mut timeline, &frame_dir, label, None, None)?;
            }
            ScriptOp::Key(token) => {
                let (key_event, display) = parse_key_token(&token)?;
                let mode_before = format!("{:?}", app.mode());
                let mapped_action = action_from_key(app.mode(), key_event);
                if let Some(action) = mapped_action.clone() {
                    let outcome = app.apply(action, &mut io);
                    if outcome == CommandOutcome::Quit {
                        quit = true;
                    }
                }
                record_frame(
                    &app,
                    &mut timeline,
                    &frame_dir,
                    None,
                    Some(display),
                    mapped_action.map(|a| format!("{a:?}")),
                )?;
                if let Some(last) = timeline.frames.last_mut() {
                    last.mode = Some(mode_before);
                }
            }
            ScriptOp::Text(text) => {
                for ch in text.chars() {
                    let key_event = make_key_event(KeyCode::Char(ch), KeyModifiers::NONE);
                    let mode_before = format!("{:?}", app.mode());
                    let mapped_action = action_from_key(app.mode(), key_event);
                    if let Some(action) = mapped_action.clone() {
                        let outcome = app.apply(action, &mut io);
                        if outcome == CommandOutcome::Quit {
                            quit = true;
                        }
                    }
                    record_frame(
                        &app,
                        &mut timeline,
                        &frame_dir,
                        None,
                        Some(ch.to_string()),
                        mapped_action.map(|a| format!("{a:?}")),
                    )?;
                    if let Some(last) = timeline.frames.last_mut() {
                        last.mode = Some(mode_before);
                    }
                    if quit {
                        break;
                    }
                }
            }
        }
    }

    let timeline_path = output_dir.join("timeline.json");
    timeline.save_json(&timeline_path)?;

    println!(
        "captured {} frames at {}x{} -> {}",
        timeline.frames.len(),
        size.width,
        size.height,
        timeline_path.display()
    );
    Ok(())
}

fn parse_script(input: &str) -> Result<Vec<ScriptOp>> {
    let mut ops = Vec::new();
    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("capture") {
            let label = rest.trim();
            if label.is_empty() {
                ops.push(ScriptOp::Capture(None));
            } else {
                ops.push(ScriptOp::Capture(Some(label.to_string())));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("key ") {
            let token = rest.trim();
            if token.is_empty() {
                bail!("line {line_no}: key requires a token");
            }
            ops.push(ScriptOp::Key(token.to_string()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("text ") {
            ops.push(ScriptOp::Text(rest.to_string()));
            continue;
        }

        bail!("line {line_no}: unknown command '{line}'");
    }
    Ok(ops)
}

fn parse_key_token(token: &str) -> Result<(KeyEvent, String)> {
    let normalized = token.trim();
    if normalized.len() == 1 {
        let ch = normalized
            .chars()
            .next()
            .ok_or_else(|| anyhow!("empty key token"))?;
        return Ok((
            make_key_event(KeyCode::Char(ch), KeyModifiers::NONE),
            ch.to_string(),
        ));
    }

    let lower = normalized.to_ascii_lowercase();
    let event = match lower.as_str() {
        "enter" => make_key_event(KeyCode::Enter, KeyModifiers::NONE),
        "esc" | "escape" => make_key_event(KeyCode::Esc, KeyModifiers::NONE),
        "tab" => make_key_event(KeyCode::Tab, KeyModifiers::NONE),
        "backtab" => make_key_event(KeyCode::BackTab, KeyModifiers::NONE),
        "backspace" => make_key_event(KeyCode::Backspace, KeyModifiers::NONE),
        "delete" | "del" => make_key_event(KeyCode::Delete, KeyModifiers::NONE),
        "left" => make_key_event(KeyCode::Left, KeyModifiers::NONE),
        "right" => make_key_event(KeyCode::Right, KeyModifiers::NONE),
        "up" => make_key_event(KeyCode::Up, KeyModifiers::NONE),
        "down" => make_key_event(KeyCode::Down, KeyModifiers::NONE),
        "f1" => make_key_event(KeyCode::F(1), KeyModifiers::NONE),
        "f2" => make_key_event(KeyCode::F(2), KeyModifiers::NONE),
        "f3" => make_key_event(KeyCode::F(3), KeyModifiers::NONE),
        _ if lower.starts_with("ctrl+") => {
            let suffix = &normalized[5..];
            if suffix.chars().count() != 1 {
                bail!("unsupported ctrl key token '{token}'");
            }
            let ch = suffix.chars().next().expect("count checked");
            make_key_event(
                KeyCode::Char(ch.to_ascii_lowercase()),
                KeyModifiers::CONTROL,
            )
        }
        _ => bail!("unsupported key token '{token}'"),
    };

    Ok((event, normalized.to_string()))
}

fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn record_frame(
    app: &App,
    timeline: &mut CaptureTimeline,
    frame_dir: &Path,
    label: Option<String>,
    key: Option<String>,
    action: Option<String>,
) -> Result<()> {
    let size = CaptureSize::new(timeline.width, timeline.height);
    let frame = capture_app_frame(app, size)?;
    let idx = timeline.frames.len();
    let stem = format!("frame_{idx:04}");
    write_frame_text(&frame, frame_dir.join(format!("{stem}.txt")), false)?;
    write_frame_json(&frame, frame_dir.join(format!("{stem}.json")))?;
    write_frame_svg(&frame, frame_dir.join(format!("{stem}.svg")))?;

    timeline.push_frame(TimelineFrame {
        label,
        mode: Some(format!("{:?}", app.mode())),
        key,
        action,
        frame,
    });
    Ok(())
}
