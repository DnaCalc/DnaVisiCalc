use std::io;
use std::path::PathBuf;
use std::time::Duration;

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
use crate::{App, CommandOutcome, FsWorkbookIo, action_from_key, render_app};

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
    let mut trace = options
        .event_trace_path
        .as_deref()
        .map(EventTrace::open)
        .transpose()?;

    let run_result = run_event_loop(&mut terminal, &mut app, &mut io, &mut trace);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    io: &mut FsWorkbookIo,
    trace: &mut Option<EventTrace>,
) -> Result<()> {
    loop {
        let term_size = terminal.size()?;
        let grid_area_height = term_size.height.saturating_sub(6).max(1);
        let (grid_width, grid_height) = compute_grid_dimensions(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: term_size.width,
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
                        let outcome = app.apply(action, io);
                        if outcome == CommandOutcome::Quit {
                            break;
                        }
                    }
                }
                Event::Resize(width, height) => {
                    let grid_area_height = height.saturating_sub(6).max(1);
                    let (grid_width, grid_height) = compute_grid_dimensions(ratatui::layout::Rect {
                        x: 0,
                        y: 0,
                        width,
                        height: grid_area_height,
                    });
                    app.set_viewport_dimensions(grid_width, grid_height);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
