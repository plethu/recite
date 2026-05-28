use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::config::Keymap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TextBuffer {
    text: String,
    cursor: usize,
}

impl TextBuffer {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub(crate) fn insert(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = self.text[..self.cursor]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0);
        self.text.drain(previous..self.cursor);
        self.cursor = previous;
    }

    pub(crate) fn delete(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let next = self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.text.len());
        self.text.drain(self.cursor..next);
    }

    pub(crate) fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0);
    }

    pub(crate) fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        self.cursor = self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.text.len());
    }

    pub(crate) fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    pub(crate) fn delete_word_before_cursor(&mut self) {
        while self.cursor > 0
            && self.text[..self.cursor]
                .chars()
                .last()
                .is_some_and(char::is_whitespace)
        {
            self.backspace();
        }
        while self.cursor > 0
            && self.text[..self.cursor]
                .chars()
                .last()
                .is_some_and(|ch| !ch.is_whitespace())
        {
            self.backspace();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PromptMode {
    Normal,
    Insert,
    Command,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TuiIntent {
    Quit,
    Submit,
    Cancel,
    MoveNext,
    MovePrevious,
    StartInsert,
    OpenCommand,
    ToggleHelp,
    ToggleDeferredQueue,
    Text(char),
    Backspace,
    Delete,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    ClearLine,
    DeleteWord,
    Ignore,
}

pub(crate) fn map_key(keymap: Keymap, mode: PromptMode, key: KeyEvent) -> TuiIntent {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => TuiIntent::Quit,
            KeyCode::Char('d') | KeyCode::Char('D') => TuiIntent::ToggleDeferredQueue,
            KeyCode::Char('u') | KeyCode::Char('U') => TuiIntent::ClearLine,
            KeyCode::Char('w') | KeyCode::Char('W') => TuiIntent::DeleteWord,
            _ => TuiIntent::Ignore,
        };
    }

    match mode {
        PromptMode::Help => match key.code {
            KeyCode::Esc | KeyCode::Char('?') => TuiIntent::Cancel,
            KeyCode::Char('q') => TuiIntent::Quit,
            _ => TuiIntent::Ignore,
        },
        PromptMode::Command => match key.code {
            KeyCode::Esc => TuiIntent::Cancel,
            KeyCode::Enter => TuiIntent::Submit,
            KeyCode::Char(ch) => TuiIntent::Text(ch),
            KeyCode::Backspace => TuiIntent::Backspace,
            KeyCode::Delete => TuiIntent::Delete,
            KeyCode::Left => TuiIntent::MoveCursorLeft,
            KeyCode::Right => TuiIntent::MoveCursorRight,
            KeyCode::Home => TuiIntent::MoveCursorStart,
            KeyCode::End => TuiIntent::MoveCursorEnd,
            _ => TuiIntent::Ignore,
        },
        PromptMode::Insert => match key.code {
            KeyCode::Esc => {
                if keymap == Keymap::Vim {
                    TuiIntent::Cancel
                } else {
                    TuiIntent::Quit
                }
            }
            KeyCode::Enter => TuiIntent::Submit,
            KeyCode::Char('?') => TuiIntent::ToggleHelp,
            KeyCode::Char(ch) => TuiIntent::Text(ch),
            KeyCode::Backspace => TuiIntent::Backspace,
            KeyCode::Delete => TuiIntent::Delete,
            KeyCode::Left => TuiIntent::MoveCursorLeft,
            KeyCode::Right => TuiIntent::MoveCursorRight,
            KeyCode::Home => TuiIntent::MoveCursorStart,
            KeyCode::End => TuiIntent::MoveCursorEnd,
            KeyCode::Up => TuiIntent::MovePrevious,
            KeyCode::Down => TuiIntent::MoveNext,
            _ => TuiIntent::Ignore,
        },
        PromptMode::Normal => match key.code {
            KeyCode::Esc => TuiIntent::Quit,
            KeyCode::Enter => TuiIntent::Submit,
            KeyCode::Up => TuiIntent::MovePrevious,
            KeyCode::Down => TuiIntent::MoveNext,
            KeyCode::Char(':') if keymap == Keymap::Vim => TuiIntent::OpenCommand,
            KeyCode::Char('?') => TuiIntent::ToggleHelp,
            KeyCode::Char('i') if keymap == Keymap::Vim => TuiIntent::StartInsert,
            KeyCode::Char('j') if keymap == Keymap::Vim => TuiIntent::MoveNext,
            KeyCode::Char('k') if keymap == Keymap::Vim => TuiIntent::MovePrevious,
            KeyCode::Char(ch) if keymap == Keymap::Standard => TuiIntent::Text(ch),
            _ => TuiIntent::Ignore,
        },
    }
}

pub(crate) fn command_quits(command: &str) -> bool {
    matches!(command.trim(), "q" | "quit")
}
