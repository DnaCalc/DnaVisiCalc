use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::event_trace::EventTrace;
use crate::render::compute_grid_dimensions;
use crate::{Action, App, CommandOutcome, FsWorkbookIo, WorkbookIo, action_from_key, render_app};

#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    pub test_exit: bool,
    pub event_trace_path: Option<PathBuf>,
}

pub fn run_from_env() -> Result<()> {
    let test_exit = std::env::var_os("DNAVISICALC_TEST_EXIT").is_some();
    let event_trace_path = std::env::var_os("DNAVISICALC_EVENT_TRACE").map(PathBuf::from);
    run_with_options(RuntimeOptions {
        test_exit,
        event_trace_path,
    })
}

pub fn run_with_options(options: RuntimeOptions) -> Result<()> {
    run_with_runner(options, run_terminal_app)
}

fn run_with_runner<F>(options: RuntimeOptions, runner: F) -> Result<()>
where
    F: FnOnce(RuntimeOptions) -> Result<()>,
{
    if options.test_exit {
        return Ok(());
    }
    runner(options)
}

fn run_terminal_app(options: RuntimeOptions) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut io = FsWorkbookIo;
    let mut clipboard = SystemClipboard::new();
    let mut trace = options
        .event_trace_path
        .as_deref()
        .map(EventTrace::open)
        .transpose()?;

    let run_result = run_event_loop(&mut terminal, &mut app, &mut io, &mut clipboard, &mut trace);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    io: &mut FsWorkbookIo,
    clipboard: &mut impl ClipboardAccess,
    trace: &mut Option<EventTrace>,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let mut volatile_accumulator: f64 = 0.0;

    loop {
        let term_size = terminal.size()?;
        let grid_area_height = term_size.height.saturating_sub(6).max(1);
        let grid_area_width = if app.has_right_panel() {
            (term_size.width as u32 * 67 / 100) as u16
        } else {
            term_size.width
        };
        let (grid_width, grid_height) = compute_grid_dimensions(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: grid_area_width,
            height: grid_area_height,
        });
        app.set_viewport_dimensions(grid_width, grid_height);

        terminal.draw(|frame| render_app(frame, app))?;

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) => {
                    let mode = app.mode();
                    let mapped_action = action_from_key(mode, key.clone());
                    if let Some(trace_writer) = trace.as_mut() {
                        trace_writer.log_key(mode, &key, mapped_action.as_ref())?;
                    }
                    if let Some(action) = mapped_action {
                        let outcome = apply_action_with_clipboard(app, io, clipboard, action);
                        if outcome == CommandOutcome::Quit {
                            break;
                        }
                    }
                }
                Event::Resize(width, height) => {
                    let grid_area_height = height.saturating_sub(6).max(1);
                    let grid_area_width = if app.has_right_panel() {
                        (width as u32 * 67 / 100) as u16
                    } else {
                        width
                    };
                    let (grid_width, grid_height) =
                        compute_grid_dimensions(ratatui::layout::Rect {
                            x: 0,
                            y: 0,
                            width: grid_area_width,
                            height: grid_area_height,
                        });
                    app.set_viewport_dimensions(grid_width, grid_height);
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let elapsed = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;

        if app.has_stream_cells() {
            app.tick_streams(elapsed);
        }

        if app.has_volatile_cells() {
            volatile_accumulator += elapsed;
            if volatile_accumulator >= 1.0 {
                volatile_accumulator -= 1.0;
                app.volatile_recalc();
            }
        } else {
            volatile_accumulator = 0.0;
        }
    }

    Ok(())
}

trait ClipboardAccess {
    fn get_text(&mut self) -> Result<String, String>;
    fn set_text(&mut self, text: &str) -> Result<(), String>;
}

struct SystemClipboard {
    inner: Option<arboard::Clipboard>,
    init_error: Option<String>,
}

impl SystemClipboard {
    fn new() -> Self {
        match arboard::Clipboard::new() {
            Ok(inner) => Self {
                inner: Some(inner),
                init_error: None,
            },
            Err(err) => Self {
                inner: None,
                init_error: Some(err.to_string()),
            },
        }
    }
}

impl ClipboardAccess for SystemClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        let inner = self.inner.as_mut().ok_or_else(|| {
            self.init_error
                .clone()
                .unwrap_or_else(|| "system clipboard unavailable".to_string())
        })?;
        inner.get_text().map_err(|err| err.to_string())
    }

    fn set_text(&mut self, text: &str) -> Result<(), String> {
        let inner = self.inner.as_mut().ok_or_else(|| {
            self.init_error
                .clone()
                .unwrap_or_else(|| "system clipboard unavailable".to_string())
        })?;
        inner
            .set_text(text.to_string())
            .map_err(|err| err.to_string())
    }
}

fn apply_action_with_clipboard(
    app: &mut App,
    io: &mut dyn WorkbookIo,
    clipboard: &mut impl ClipboardAccess,
    action: Action,
) -> CommandOutcome {
    match action {
        Action::CopySelection => {
            let outcome = app.apply(Action::CopySelection, io);
            if outcome == CommandOutcome::Quit {
                return outcome;
            }
            let text = app.last_copy_text().map(ToString::to_string);
            if let Some(text) = text {
                if let Err(err) = clipboard.set_text(&text) {
                    app.set_status(format!("Copy error: {err}"));
                }
            }
            outcome
        }
        Action::PasteFromClipboard => match clipboard.get_text() {
            Ok(text) => app.apply(Action::BeginPasteFromClipboard(text), io),
            Err(err) => {
                app.set_status(format!("Paste error: {err}"));
                CommandOutcome::Continue
            }
        },
        other => app.apply(other, io),
    }
}

#[cfg(test)]
mod tests {
    use crate::io::MemoryWorkbookIo;

    use super::*;

    #[derive(Default)]
    struct TestClipboard {
        text: Option<String>,
        get_error: Option<String>,
        set_error: Option<String>,
    }

    impl ClipboardAccess for TestClipboard {
        fn get_text(&mut self) -> Result<String, String> {
            if let Some(err) = &self.get_error {
                return Err(err.clone());
            }
            self.text
                .clone()
                .ok_or_else(|| "clipboard empty".to_string())
        }

        fn set_text(&mut self, text: &str) -> Result<(), String> {
            if let Some(err) = &self.set_error {
                return Err(err.clone());
            }
            self.text = Some(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_exit_option_short_circuits_runtime() {
        let result = run_with_options(RuntimeOptions {
            test_exit: true,
            event_trace_path: None,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn non_test_exit_path_uses_runner() {
        let result = run_with_runner(
            RuntimeOptions {
                test_exit: false,
                event_trace_path: None,
            },
            |_| Ok(()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn non_test_exit_path_propagates_runner_error() {
        let result = run_with_runner(
            RuntimeOptions {
                test_exit: false,
                event_trace_path: None,
            },
            |_| Err(anyhow::anyhow!("runner failed")),
        );
        assert!(result.is_err());
    }

    #[test]
    fn copy_action_writes_to_clipboard() {
        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();
        let mut clipboard = TestClipboard::default();

        app.apply(Action::StartEdit, &mut io);
        app.apply(Action::InputChar('4'), &mut io);
        app.apply(Action::InputChar('2'), &mut io);
        app.apply(Action::Submit, &mut io);

        let outcome =
            apply_action_with_clipboard(&mut app, &mut io, &mut clipboard, Action::CopySelection);
        assert_eq!(outcome, CommandOutcome::Continue);
        assert_eq!(clipboard.text.as_deref(), Some("42"));
    }

    #[test]
    fn paste_action_reads_clipboard_and_enters_paste_special_mode() {
        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();
        let mut clipboard = TestClipboard {
            text: Some("1\t2".to_string()),
            ..Default::default()
        };

        let outcome = apply_action_with_clipboard(
            &mut app,
            &mut io,
            &mut clipboard,
            Action::PasteFromClipboard,
        );
        assert_eq!(outcome, CommandOutcome::Continue);
        assert_eq!(app.mode(), crate::AppMode::PasteSpecial);
    }
}
