//! Transient native lookup keeps the mounted manuscript and its input history intact.
mod choices;
mod commands;
mod index;
mod previews;
mod wording;
use super::*;
pub use crate::rules::selection;
use recite_config::WriterCommand;
use recite_writer_session::commands::targets;
use recite_writer_session::location::Location;
use std::ops::Range;
pub use wording::{empty_message, hint};
/// One opening has a distinct authority even when its mode and query are identical.
#[derive(Clone, Default)]
pub struct PickerInstance(Arc<()>);
impl PartialEq for PickerInstance {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
/// A change to the query, result set or active result; independent of command freshness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PickerRevision(u64);
impl PickerRevision {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Commands,
    GoTo,
    ProjectText,
    Passages,
    RulesChoices,
    Destinations,
    Speakers,
    RenameBeat,
}
pub use super::Search;
pub enum Purpose {
    Search(Search),
    Editor(Box<crate::choices::Request>),
}
impl Purpose {
    pub fn mode(&self) -> Mode {
        match self {
            Self::Search(Search::Commands) => Mode::Commands,
            Self::Search(Search::GoTo) => Mode::GoTo,
            Self::Search(Search::ProjectText) => Mode::ProjectText,
            Self::Search(Search::Passages) => Mode::Passages,
            Self::Editor(request) => match request.as_ref() {
                crate::choices::Request::Rules(_) => Mode::RulesChoices,
                crate::choices::Request::Destination(_) => Mode::Destinations,
                crate::choices::Request::Speaker(_) => Mode::Speakers,
                crate::choices::Request::Rename(_) => Mode::RenameBeat,
            },
        }
    }
}
#[derive(Clone)]
pub enum Item {
    Command(WriterCommand),
    Location(Location),
    Passage {
        location: Location,
        passage: recite_writer_model::Passage,
    },
    Lookup(targets::Target),
    Rule(selection::Choice),
    Destination(String),
    Speaker(String),
    RenameBeat(String),
}
pub struct Entry {
    pub item: Item,
    pub title: String,
    pub detail: String,
}
pub struct Picker {
    pub purpose: Purpose,
    pub(super) revision: PickerRevision,
    pub instance: PickerInstance,
    pub query: String,
    pub entries: Vec<Entry>,
    pub matches: Vec<usize>,
    pub active: usize,
    limit: usize,
    pub total: usize,
    pub previews: previews::Previews,
    /// The results the list shows, as it last reported them.
    visible: Range<usize>,
    /// The first result the list shows; it moves only to keep the active result in view.
    pub anchor: usize,
    /// Where a combobox slip hangs from the label that opened it, in window pixels; a picker
    /// opened any other way sits below the tab row.
    pub placement: Option<(u32, u32)>,
}
impl Picker {
    pub fn new(purpose: Purpose, entries: Vec<Entry>) -> Self {
        let matches = (0..entries.len()).collect();
        let mut picker = Self {
            purpose,
            instance: PickerInstance::default(),
            revision: PickerRevision::default(),
            query: String::new(),
            total: entries.len(),
            entries,
            matches,
            active: 0,
            limit: 100,
            previews: Default::default(),
            visible: 0..0,
            anchor: 0,
            placement: None,
        };
        picker.prepare_previews();
        picker
    }
    pub fn mode(&self) -> Mode {
        self.purpose.mode()
    }
    fn step(&mut self, forward: bool) {
        let count = self.matches.len();
        if count != 0 {
            self.select((self.active + if forward { 1 } else { count - 1 }) % count);
        }
    }
    fn select(&mut self, active: usize) {
        self.active = active;
        self.revision.advance();
        // Keep the active result in the list: bring it in at the edge it left by, or to the top
        // before the list has reported what it shows.
        if !self.visible.contains(&active) {
            self.anchor = if self.visible.is_empty() || active < self.visible.start {
                active
            } else {
                active + 1 - self.visible.len()
            };
        }
    }
    fn prepare_previews(&mut self) {
        if self.mode() == Mode::Passages {
            self.previews
                .prepare(&self.entries, &self.matches, &self.query);
        }
    }
}
impl Shell {
    pub fn switch_search(&mut self, search: Search) {
        let Some(picker) = self
            .picker
            .as_ref()
            .filter(|picker| matches!(picker.purpose, Purpose::Search(_)))
        else {
            return;
        };
        let query = picker.query.clone();
        self.open_picker(search);
        self.filter_picker(query);
    }
    pub fn open_picker(&mut self, search: Search) {
        if self.settings_open
            || self.show_welcome
            || self
                .writer
                .as_ref()
                .is_none_or(|writer| writer.phase != Phase::Writing)
        {
            return;
        }
        match index::entries(self, search) {
            Ok(entries) => self.picker = Some(Picker::new(Purpose::Search(search), entries)),
            Err(error) => self.status = error,
        }
    }
    pub fn dismiss_picker(&mut self) {
        self.picker = None;
    }
    pub fn step_picker(&mut self, forward: bool) {
        if let Some(picker) = &mut self.picker {
            picker.step(forward);
        }
    }
    /// The list scrolled to show `visible`.
    pub fn picker_scrolled(&mut self, visible: Range<usize>) {
        if let Some(picker) = &mut self.picker {
            picker.visible = visible;
        }
    }
    pub fn filter_picker(&mut self, query: String) {
        if let Some(picker) = &mut self.picker {
            picker.query = query;
            picker.active = 0;
            picker.anchor = 0;
            picker.limit = 100;
        }
        self.refresh_picker();
    }
    pub fn submit_picker(&mut self) {
        let active = self.picker.as_ref().map_or(0, |picker| picker.active);
        self.activate_picker(active);
    }
    pub fn load_picker_matches(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.limit += 100;
        }
        self.refresh_picker();
    }
    fn refresh_picker(&mut self) {
        let search = self
            .writer
            .as_ref()
            .and_then(|writer| writer.session.project())
            .map(|project| project.files().search_index());
        if let Some(picker) = &mut self.picker {
            picker.refresh(self.preferences.config.ui.keymap, search.as_deref());
        }
    }
    pub fn activate_picker(&mut self, index: usize) {
        let item = self
            .picker
            .as_ref()
            .and_then(|picker| {
                picker
                    .matches
                    .get(index)
                    .and_then(|index| picker.entries.get(*index))
            })
            .map(|entry| entry.item.clone());
        match item {
            Some(Item::Command(command)) if self.command_enabled(command) => {
                self.dismiss_picker();
                self.run_command(command);
            }
            Some(Item::Lookup(target)) => {
                let query = self
                    .picker
                    .as_ref()
                    .map(|picker| picker.query.clone())
                    .unwrap_or_default();
                if let Some(writer) = &mut self.writer {
                    writer.lookup(query, target);
                    if writer.navigation.busy() {
                        self.dismiss_picker();
                    }
                }
            }
            Some(Item::Location(location) | Item::Passage { location, .. }) => {
                if let Some(writer) = &mut self.writer {
                    writer.navigate(crate::navigation::Target::Location(location));
                    if writer.navigation.busy() {
                        self.dismiss_picker();
                    }
                }
            }
            Some(
                item @ (Item::Rule(_)
                | Item::Destination(_)
                | Item::Speaker(_)
                | Item::RenameBeat(_)),
            ) => self.activate_editor_choice(item),
            _ => {}
        }
    }
}
