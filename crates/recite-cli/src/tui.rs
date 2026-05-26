use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        is_raw_mode_enabled,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde::Deserialize;

use crate::{args::PlayKeymap, error::CliError, i18n::UiLocale};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Keymap {
    Standard,
    Vim,
}

impl From<PlayKeymap> for Keymap {
    fn from(keymap: PlayKeymap) -> Self {
        match keymap {
            PlayKeymap::Standard => Self::Standard,
            PlayKeymap::Vim => Self::Vim,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum KeyHints {
    #[default]
    Contextual,
    Compact,
    Hidden,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TuiSettings {
    pub(crate) locale: UiLocale,
    pub(crate) keymap: Keymap,
    pub(crate) key_hints: KeyHints,
    pub(crate) show_unavailable_choices: bool,
}

impl Default for TuiSettings {
    fn default() -> Self {
        Self {
            locale: UiLocale::default(),
            keymap: Keymap::Standard,
            key_hints: KeyHints::Contextual,
            show_unavailable_choices: true,
        }
    }
}

impl TuiSettings {
    pub(crate) fn load(keymap_override: Option<PlayKeymap>) -> Result<Self, CliError> {
        let mut settings = match config_path() {
            Some(path) if path.exists() => Self::load_path(&path)?,
            _ => Self::default(),
        };
        if let Some(keymap) = keymap_override {
            settings.keymap = keymap.into();
        }
        Ok(settings)
    }

    fn load_path(path: &Path) -> Result<Self, CliError> {
        let source = fs::read_to_string(path).map_err(|source| CliError::TuiConfigRead {
            path: path.to_owned(),
            source,
        })?;
        let raw =
            toml::from_str::<RawConfig>(&source).map_err(|source| CliError::TuiConfigToml {
                path: path.to_owned(),
                source,
            })?;
        raw.into_settings(path)
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RECITE_CONFIG") {
        return Some(PathBuf::from(path));
    }
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Some(
            PathBuf::from(config_home)
                .join("recite")
                .join("config.toml"),
        );
    }
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("recite")
            .join("config.toml")
    })
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    ui: RawUiConfig,
    #[serde(default)]
    play: RawPlayConfig,
}

impl RawConfig {
    fn into_settings(self, path: &Path) -> Result<TuiSettings, CliError> {
        let defaults = TuiSettings::default();
        let locale = match self.ui.locale {
            Some(locale) => UiLocale::parse(&locale).map_err(|()| CliError::UiLocaleInvalid {
                path: path.to_owned(),
                locale,
            })?,
            None => defaults.locale,
        };
        Ok(TuiSettings {
            locale,
            keymap: self.ui.keymap.unwrap_or(defaults.keymap),
            key_hints: self.ui.key_hints.unwrap_or(defaults.key_hints),
            show_unavailable_choices: self
                .play
                .show_unavailable_choices
                .unwrap_or(defaults.show_unavailable_choices),
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUiConfig {
    locale: Option<String>,
    keymap: Option<Keymap>,
    key_hints: Option<KeyHints>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlayConfig {
    show_unavailable_choices: Option<bool>,
}

impl<'de> Deserialize<'de> for Keymap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "standard" => Ok(Self::Standard),
            "vim" => Ok(Self::Vim),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["standard", "vim"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for KeyHints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "contextual" => Ok(Self::Contextual),
            "compact" => Ok(Self::Compact),
            "hidden" => Ok(Self::Hidden),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["contextual", "compact", "hidden"],
            )),
        }
    }
}

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
            KeyCode::Char(':') if keymap == Keymap::Standard => TuiIntent::OpenCommand,
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
            KeyCode::Char(':') => TuiIntent::OpenCommand,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_buffer_edits_with_cursor_awareness() {
        let mut buffer = TextBuffer::default();
        buffer.insert('a');
        buffer.insert('c');
        buffer.move_left();
        buffer.insert('b');
        assert_eq!(buffer.as_str(), "abc");
        buffer.delete_word_before_cursor();
        assert_eq!(buffer.as_str(), "c");
        buffer.move_end();
        buffer.backspace();
        assert_eq!(buffer.as_str(), "");
    }

    #[test]
    fn maps_standard_printable_keys_to_text() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(
            map_key(Keymap::Standard, PromptMode::Normal, key),
            TuiIntent::Text('j')
        );
    }

    #[test]
    fn maps_vim_navigation_keys_in_normal_mode() {
        let down = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let up = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(
            map_key(Keymap::Vim, PromptMode::Normal, down),
            TuiIntent::MoveNext
        );
        assert_eq!(
            map_key(Keymap::Vim, PromptMode::Normal, up),
            TuiIntent::MovePrevious
        );
    }

    #[test]
    fn command_parser_accepts_quit_commands_only() {
        assert!(command_quits("q"));
        assert!(command_quits("quit"));
        assert!(!command_quits("write"));
    }

    #[test]
    fn settings_default_to_standard_contextual_hints_and_visible_unavailable_choices() {
        let settings = TuiSettings::default();

        assert_eq!(settings.keymap, Keymap::Standard);
        assert_eq!(settings.key_hints, KeyHints::Contextual);
        assert_eq!(settings.locale.to_string(), crate::i18n::DEFAULT_LOCALE);
        assert!(settings.show_unavailable_choices);
    }

    #[test]
    fn settings_parse_toml_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"[ui]
locale = "en-GB"
keymap = "vim"
key_hints = "compact"

[play]
show_unavailable_choices = false
"#,
        )
        .expect("write config");

        let settings = TuiSettings::load_path(&path).expect("config loads");

        assert_eq!(settings.locale.to_string(), "en-GB");
        assert_eq!(settings.keymap, Keymap::Vim);
        assert_eq!(settings.key_hints, KeyHints::Compact);
        assert!(!settings.show_unavailable_choices);
    }

    #[test]
    fn settings_reject_unknown_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"[ui]
keymap = "emacs"
"#,
        )
        .expect("write config");

        let error = TuiSettings::load_path(&path).expect_err("config fails");

        assert!(error.to_string().contains("failed to parse UI config"));
    }

    #[test]
    fn settings_reject_malformed_locale() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"[ui]
locale = "not a locale"
"#,
        )
        .expect("write config");

        let error = TuiSettings::load_path(&path).expect_err("config fails");

        assert!(error.to_string().contains("invalid [ui].locale"));
    }
}
