//! Transient native lookup keeps the mounted manuscript and its input history intact.
mod choices;
mod commands;
mod index;
mod lifecycle;
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
    /// Whether every result of a choice slip is of one kind, which then goes unsaid on each line.
    pub fn one_kind(&self) -> bool {
        matches!(self.purpose, Purpose::Editor(_))
            && self
                .entries
                .windows(2)
                .all(|pair| pair[0].detail == pair[1].detail)
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
