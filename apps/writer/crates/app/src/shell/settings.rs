//! Personal preferences, persisted through the shared `recite_config` user configuration.
#[cfg(test)]
mod tests;
use super::*;
use recite_config::{UserConfigEdit, WriterCommand, WriterPresentationField, WriterShortcut};
use std::collections::BTreeMap;

pub struct Settings {
    pub keyboard: bool,
    pub command: WriterCommand,
    pub drafts: BTreeMap<WriterCommand, String>,
    pub page: usize,
    pub query: String,
    pub feedback: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            keyboard: false,
            command: WriterCommand::Save,
            drafts: BTreeMap::new(),
            page: 0,
            query: String::new(),
            feedback: None,
        }
    }
}
pub const COMMANDS_PER_PAGE: usize = 8;
fn commands() -> impl Iterator<Item = WriterCommand> {
    WriterCommand::ALL
        .iter()
        .copied()
        .filter(|command| command.rebindable())
}
pub fn command_label(command: WriterCommand) -> String {
    if command == WriterCommand::Source {
        "Open source in editor".into()
    } else {
        recite_writer_session::commands::label(command, false)
    }
}
impl Shell {
    pub fn matching_shortcuts(&self) -> impl Iterator<Item = WriterCommand> + use<> {
        let query = self.settings.query.trim().to_lowercase();
        commands().filter(move |command| command_label(*command).to_lowercase().contains(&query))
    }
    pub fn search_shortcuts(&mut self, query: String) {
        self.settings.query = query;
        self.settings.page = 0;
    }
    pub fn open_project_settings(&mut self) {
        if let Some(writer) = &mut self.writer {
            writer.open_project_workspace(crate::workspaces::Open::Settings);
            self.settings_open = false;
            self.show_welcome = false;
        }
    }
    pub(crate) fn show_settings(&mut self) {
        self.settings_open = true;
        self.settings.page = self
            .matching_shortcuts()
            .position(|command| command == self.settings.command)
            .unwrap_or(0)
            / COMMANDS_PER_PAGE;
    }
    fn store_preference(&mut self, edit: UserConfigEdit) -> Result<(), String> {
        self.preferences.update(edit)?;
        if let Some(writer) = &mut self.writer {
            writer.configure(self.preferences.config.clone());
        }
        Ok(())
    }
    pub fn update_preference(&mut self, edit: UserConfigEdit) {
        match self.store_preference(edit) {
            Ok(()) => self.status = "Preferences saved.".into(),
            Err(error) => self.status = format!("Could not save preferences: {error}"),
        }
    }
    pub fn adjust_presentation(&mut self, field: WriterPresentationField, increase: bool) {
        let presentation = self.preferences.config.writer.presentation;
        let step = match field {
            WriterPresentationField::UiScale
            | WriterPresentationField::DrawerWidth
            | WriterPresentationField::ScriptWidth => 10,
            _ => 1,
        };
        let value = if increase {
            presentation.value(field).checked_add(step)
        } else {
            presentation.value(field).checked_sub(step)
        };
        let Some(value) = value else {
            return;
        };
        match presentation.with_value(field, value) {
            Ok(next) => self.update_preference(UserConfigEdit::WriterPresentation(next)),
            Err(error) => self.status = error.to_string(),
        }
    }
    pub fn choose_shortcut(&mut self, command: WriterCommand) {
        self.settings.command = command;
        self.settings.feedback = None;
    }
    pub fn shortcut_draft(&self) -> &str {
        self.settings
            .drafts
            .get(&self.settings.command)
            .map(String::as_str)
            .unwrap_or_else(|| {
                self.preferences
                    .config
                    .writer
                    .shortcuts
                    .binding(self.settings.command)
            })
    }
    pub fn edit_shortcut(&mut self, value: String) {
        self.settings.drafts.insert(self.settings.command, value);
        self.settings.feedback = None;
    }
    pub fn reset_shortcuts(&mut self) {
        match self.store_preference(UserConfigEdit::WriterShortcuts(Default::default())) {
            Ok(()) => {
                self.settings.drafts.clear();
                self.settings.feedback = Some("All shortcuts reset to their defaults.".into());
            }
            Err(error) => {
                self.settings.feedback = Some(format!("Could not reset shortcuts: {error}"))
            }
        }
    }
    pub fn save_shortcut(&mut self) {
        let result =
            WriterShortcut::try_from(self.shortcut_draft().to_owned()).and_then(|binding| {
                let mut shortcuts = self.preferences.config.writer.shortcuts.clone();
                shortcuts.rebind(self.settings.command, binding)?;
                Ok(shortcuts)
            });
        let result = result
            .map_err(|error| error.to_string())
            .and_then(|shortcuts| {
                self.store_preference(UserConfigEdit::WriterShortcuts(shortcuts))
            });
        self.settings.feedback = Some(match result {
            Ok(()) => {
                self.settings.drafts.remove(&self.settings.command);
                format!(
                    "Shortcut saved for {}.",
                    command_label(self.settings.command)
                )
            }
            Err(error) => format!("Shortcut not saved: {error}"),
        });
    }
}
