use std::io;

use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        is_raw_mode_enabled,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::error::CliError;

pub(crate) fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), CliError> {
    if is_raw_mode_enabled()? {
        disable_raw_mode()?;
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub(crate) fn enter_terminal() -> Result<TerminalRestoreGuard, CliError> {
    enable_raw_mode()?;
    let mut restore_guard = TerminalRestoreGuard::new();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    restore_guard.entered_alternate_screen();
    Ok(restore_guard)
}

pub(crate) struct TerminalRestoreGuard {
    active: bool,
    entered_alternate_screen: bool,
}

impl TerminalRestoreGuard {
    fn new() -> Self {
        Self {
            active: true,
            entered_alternate_screen: false,
        }
    }

    fn entered_alternate_screen(&mut self) {
        self.entered_alternate_screen = true;
    }

    pub(crate) fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if is_raw_mode_enabled().unwrap_or(false) {
            let _ = disable_raw_mode();
        }
        if self.entered_alternate_screen {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
        }
    }
}
