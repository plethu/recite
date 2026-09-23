//! Portable workspace shortcuts, validated before loading or saving.
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterCommand {
    Script,
    Map,
    Source,
    Split,
    Focus,
    Commands,
    GoTo,
    OpenProject,
    Save,
    SaveAll,
    Apply,
    Undo,
    Redo,
    AddBeat,
    AddLine,
    AddReply,
    Preview,
    Localise,
    Declarations,
    Build,
    Rename,
    Settings,
    Back,
    Forward,
    Find,
    NextMatch,
    PreviousMatch,
    PaneLeft,
    PaneRight,
    PaneUp,
    PaneDown,
    Close,
}
impl WriterCommand {
    pub const ALL: &[Self] = &[
        Self::Script,
        Self::Map,
        Self::Source,
        Self::Split,
        Self::Focus,
        Self::Commands,
        Self::GoTo,
        Self::OpenProject,
        Self::Save,
        Self::SaveAll,
        Self::Apply,
        Self::Undo,
        Self::Redo,
        Self::AddBeat,
        Self::AddLine,
        Self::AddReply,
        Self::Preview,
        Self::Localise,
        Self::Declarations,
        Self::Build,
        Self::Rename,
        Self::Settings,
        Self::Back,
        Self::Forward,
        Self::Find,
        Self::NextMatch,
        Self::PreviousMatch,
        Self::PaneLeft,
        Self::PaneRight,
        Self::PaneUp,
        Self::PaneDown,
        Self::Close,
    ];
    pub const fn key(self) -> &'static str {
        match self {
            Self::Script => "script",
            Self::Map => "map",
            Self::Source => "source",
            Self::Split => "split",
            Self::Focus => "focus",
            Self::Commands => "commands",
            Self::GoTo => "go_to",
            Self::OpenProject => "open_project",
            Self::Save => "save",
            Self::SaveAll => "save_all",
            Self::Apply => "apply",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::AddBeat => "add_beat",
            Self::AddLine => "add_line",
            Self::AddReply => "add_reply",
            Self::Preview => "preview",
            Self::Localise => "localise",
            Self::Declarations => "declarations",
            Self::Build => "build",
            Self::Rename => "rename",
            Self::Settings => "settings",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Find => "find",
            Self::NextMatch => "next_match",
            Self::PreviousMatch => "previous_match",
            Self::PaneLeft => "pane_left",
            Self::PaneRight => "pane_right",
            Self::PaneUp => "pane_up",
            Self::PaneDown => "pane_down",
            Self::Close => "close",
        }
    }
    pub const fn rebindable(self) -> bool {
        !matches!(self, Self::Apply | Self::Undo | Self::Redo)
    }
    pub const fn default_shortcut(self) -> &'static str {
        match self {
            Self::Commands => "Primary+Shift+P",
            Self::GoTo => "Primary+P",
            Self::Save => "Primary+S",
            Self::SaveAll => "Primary+Shift+S",
            Self::Settings => "Primary+,",
            Self::Script => "Primary+1",
            Self::Map => "Primary+2",
            Self::Source => "Primary+3",
            Self::Focus => "Primary+Shift+F",
            Self::Split => "Primary+Shift+L",
            _ => "",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct ShortcutError(String);

/// A portable chord. Empty means disabled; Primary is Command on macOS and Ctrl elsewhere.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub struct WriterShortcut(String);
impl WriterShortcut {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for WriterShortcut {
    type Error = ShortcutError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self(value));
        }
        let (prefix, key) = value.rsplit_once('+').unwrap_or(("", value.as_str()));
        let modifiers: Vec<_> = if prefix.is_empty() {
            Vec::new()
        } else {
            prefix.split('+').collect()
        };
        let function = key
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n) && n != 6 && format!("F{n}") == key);
        let character = key.len() == 1
            && key
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b',');
        let canonical: Vec<_> = ["Primary", "Alt", "Shift"]
            .into_iter()
            .filter(|m| modifiers.contains(m))
            .collect();
        if value.starts_with('+')
            || canonical != modifiers
            || !(function || character && modifiers.contains(&"Primary"))
        {
            return Err(ShortcutError("Use Primary+Shift+K, Primary+1, or F1–F12 (except F6). Primary means Ctrl or Cmd. Leave blank to disable.".into()));
        }
        if [
            "Primary+Q",
            "Primary+C",
            "Primary+V",
            "Primary+X",
            "Primary+A",
            "Primary+Z",
            "Primary+Y",
            "Primary+Shift+Z",
            "Alt+F4",
        ]
        .contains(&value.as_str())
        {
            return Err(ShortcutError(
                "That combination belongs to text editing or the operating system.".into(),
            ));
        }
        Ok(Self(value))
    }
}

/// Overrides are checked against defaults too, so loading cannot introduce ambiguous dispatch.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(try_from = "BTreeMap<WriterCommand, WriterShortcut>")]
pub struct WriterShortcuts(BTreeMap<WriterCommand, WriterShortcut>);
impl WriterShortcuts {
    pub fn binding(&self, command: WriterCommand) -> &str {
        self.0
            .get(&command)
            .map_or(command.default_shortcut(), WriterShortcut::as_str)
    }
    pub fn overrides(&self) -> impl Iterator<Item = (WriterCommand, &WriterShortcut)> {
        self.0.iter().map(|(command, binding)| (*command, binding))
    }
    pub fn rebind(
        &mut self,
        command: WriterCommand,
        binding: WriterShortcut,
    ) -> Result<(), ShortcutError> {
        let mut next = self.0.clone();
        next.insert(command, binding);
        *self = Self::try_from(next)?;
        Ok(())
    }
}
impl TryFrom<BTreeMap<WriterCommand, WriterShortcut>> for WriterShortcuts {
    type Error = ShortcutError;
    fn try_from(value: BTreeMap<WriterCommand, WriterShortcut>) -> Result<Self, Self::Error> {
        if value.keys().any(|c| !c.rebindable()) {
            return Err(ShortcutError(
                "Apply, Undo and Redo belong to the focused editor.".into(),
            ));
        }
        let result = Self(value);
        let mut seen = BTreeMap::new();
        for command in WriterCommand::ALL {
            let binding = result.binding(*command);
            if !binding.is_empty()
                && let Some(other) = seen.insert(binding, command)
            {
                return Err(ShortcutError(format!(
                    "{binding} is assigned to both {} and {}. Disable or change one first.",
                    other.key(),
                    command.key()
                )));
            }
        }
        Ok(result)
    }
}
