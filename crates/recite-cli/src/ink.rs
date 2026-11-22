//! Terminal inks for human output: Recite's two-ink identity mapped onto the ANSI palette.
//!
//! Plum (ANSI magenta) marks what the reader acts on: carets, the selected choice and speakers.
//! Teal (ANSI cyan) draws rules, labels and locations, as the second ink does on paper. Words a
//! reader must read stay in the terminal's own foreground and are never faint, because faint text
//! fails contrast on light terminal themes. Colour follows each stream's terminal check and the
//! shared `NO_COLOR`/`CLICOLOR` settings; structured output never passes through here.
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static STDOUT: AtomicBool = AtomicBool::new(false);
static STDERR: AtomicBool = AtomicBool::new(false);
static STDERR_COLUMNS: AtomicUsize = AtomicUsize::new(0);

/// Lines wrap to the terminal up to this width; wider lines stop being comfortable to read.
const MAX_COLUMNS: usize = 100;

/// Set once by the binary. Library callers and tests keep the plain, unwrapped default.
pub(crate) fn configure(stdout: bool, stderr: bool, stderr_columns: Option<usize>) {
    STDOUT.store(stdout, Ordering::Relaxed);
    STDERR.store(stderr, Ordering::Relaxed);
    STDERR_COLUMNS.store(
        stderr_columns.map_or(0, |columns| columns.min(MAX_COLUMNS)),
        Ordering::Relaxed,
    );
}

/// The width human messages on stderr wrap to, when stderr is a terminal.
pub(crate) fn stderr_columns() -> Option<usize> {
    match STDERR_COLUMNS.load(Ordering::Relaxed) {
        0 => None,
        columns => Some(columns),
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Ink {
    color: bool,
}

impl Ink {
    pub(crate) fn stdout() -> Self {
        Self {
            color: STDOUT.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn stderr() -> Self {
        Self {
            color: STDERR.load(Ordering::Relaxed),
        }
    }

    fn sgr(self, code: &str, text: impl Display) -> String {
        if self.color {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_string()
        }
    }

    pub(crate) fn error(self, text: impl Display) -> String {
        self.sgr("1;31", text)
    }

    pub(crate) fn warning(self, text: impl Display) -> String {
        self.sgr("1;33", text)
    }

    pub(crate) fn strong(self, text: impl Display) -> String {
        self.sgr("1", text)
    }

    /// Plum: what to act on.
    pub(crate) fn plum(self, text: impl Display) -> String {
        self.sgr("1;35", text)
    }

    /// Teal: rules, gutters, labels and locations.
    pub(crate) fn teal(self, text: impl Display) -> String {
        self.sgr("36", text)
    }

    /// Teal labels that lead a line, such as `ok` and `help:`.
    pub(crate) fn teal_strong(self, text: impl Display) -> String {
        self.sgr("1;36", text)
    }

    pub(crate) fn path(self, text: impl Display) -> String {
        self.sgr("4", text)
    }
}

#[cfg(test)]
pub(crate) mod tests;
