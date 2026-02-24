use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::{App, CommandOutcome, FsWorkbookIo, action_from_key, render_app};

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeOptions {
    pub test_exit: bool,
}

pub fn run_from_env() -> Result<()> {
    let test_exit = std::env::var_os("DNAVISICALC_TEST_EXIT").is_some();
    run_with_options(RuntimeOptions { test_exit })
}

pub fn run_with_options(options: RuntimeOptions) -> Result<()> {
    run_with_runner(options, run_terminal_app)
}

fn run_with_runner<F>(options: RuntimeOptions, runner: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    if options.test_exit {
        return Ok(());
    }
    runner()
}

fn run_terminal_app() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut io = FsWorkbookIo;

    let run_result = run_event_loop(&mut terminal, &mut app, &mut io);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    io: &mut FsWorkbookIo,
) -> Result<()> {
    loop {
        terminal.draw(|frame| render_app(frame, app))?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = action_from_key(app.mode(), key) {
                    let outcome = app.apply(action, io);
                    if outcome == CommandOutcome::Quit {
                        break;
                    }
                }
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
        let result = run_with_options(RuntimeOptions { test_exit: true });
        assert!(result.is_ok());
    }

    #[test]
    fn non_test_exit_path_uses_runner() {
        let result = run_with_runner(RuntimeOptions { test_exit: false }, || Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn non_test_exit_path_propagates_runner_error() {
        let result = run_with_runner(RuntimeOptions { test_exit: false }, || {
            Err(anyhow::anyhow!("runner failed"))
        });
        assert!(result.is_err());
    }
}
