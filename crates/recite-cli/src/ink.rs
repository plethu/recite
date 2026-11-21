//! Terminal inks for human output: Recite's two-ink identity mapped onto the ANSI palette.
//!
//! Plum (ANSI magenta) marks what the reader acts on: carets, the selected choice and speakers.
//! Teal (ANSI cyan) draws rules, labels and locations, as the second ink does on paper. Words a
//! reader must read stay in the terminal's own foreground and are never faint, because faint text
//! fails contrast on light terminal themes. Colour follows each stream's terminal check and the
//! shared `NO_COLOR`/`CLICOLOR` settings; structured output never passes through here.
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

static STDOUT: AtomicBool = AtomicBool::new(false);
static STDERR: AtomicBool = AtomicBool::new(false);

/// Set once by the binary. Library callers and tests keep the plain default.
pub(crate) fn configure(stdout: bool, stderr: bool) {
    STDOUT.store(stdout, Ordering::Relaxed);
    STDERR.store(stderr, Ordering::Relaxed);
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

    #[cfg(test)]
    pub(crate) const fn coloured() -> Self {
        Self { color: true }
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
