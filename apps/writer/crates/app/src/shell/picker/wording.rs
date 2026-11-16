//! What a picker says: its title, the keys it answers to, its result hint and its empty message.
use super::*;

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Self::Commands => "Commands",
            Self::GoTo => "Go to scene or beat",
            Self::ProjectText => "Find project text",
            Self::Passages => "Find in this beat",
            Self::RulesChoices => "Choose reply rule value",
            Self::Destinations => "Change destination",
            Self::Speakers => "Change speaker",
            Self::RenameBeat => "Choose a beat to rename",
        }
    }
}
impl Mode {
    /// What the query field says before anything is typed.
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Commands => "Find a command",
            Self::GoTo => "Find scene or beat",
            Self::ProjectText => "Search saved project words",
            Self::Passages => "Find in this beat",
            Self::RulesChoices => "Find a value",
            Self::Destinations => "Find destination",
            Self::Speakers => "Find speaker",
            Self::RenameBeat => "Find beat",
        }
    }
    /// The keys this search answers to.
    pub fn keyboard_help(self) -> &'static str {
        match self {
            Mode::Commands => "Up/Down choose · Enter runs · Esc returns",
            Mode::RulesChoices | Mode::Destinations | Mode::Speakers | Mode::RenameBeat => {
                "Up/Down choose · Enter picks · Esc returns"
            }
            _ => "Up/Down choose · Enter opens · Esc returns",
        }
    }
}
/// The picker's one hint line: how many results there are, then its keys.
pub fn hint(picker: Option<&Picker>) -> String {
    let count = picker.map_or(0, |picker| picker.matches.len());
    let total = picker
        .filter(|picker| picker.total > count)
        .map_or_else(String::new, |picker| format!(" of {}", picker.total));
    let mode = picker.map_or(Mode::Commands, Picker::mode);
    format!(
        "{count}{total} {} · {}",
        if count == 1 { "result" } else { "results" },
        mode.keyboard_help()
    )
}
/// The message shown when a search has no visible matches.
pub fn empty_message(mode: Mode, picker: Option<&Picker>) -> &'static str {
    if mode == Mode::ProjectText && picker.is_some_and(|picker| picker.query.trim().is_empty()) {
        "Enter words to search the saved project."
    } else {
        "No matching results."
    }
}
