//! Translation edits use the shared file-backed catalogue, with one native presentation owner.
pub mod beat;
mod caption;
mod closing;
mod commands;
mod creation;
mod editing;
mod history;
mod keyboard;
pub mod location;
pub mod presentation;
pub mod refresh;
pub mod slot;
pub mod source_navigation;
#[cfg(any(test, feature = "test-support"))]
pub mod tests;
mod watching;
pub mod work;
pub use crate::{
    Phase, Writer,
    editor_work::AfterSave as SourceAfterSave,
    workspaces::{Screen, text},
};
pub use keyboard::TranslationTarget;
use recite_core::po::PoEntryId;
use recite_writer_session::localisation::status::TranslationStatus;
use recite_writer_session::{
    comparison::ComparisonRow,
    localisation::catalogue::{Catalogue, EntryDetails, ExternalDecision, Inspection},
};
pub use slot::{Identity, Slot};
pub(crate) use source_navigation::SourceIntent;
use std::{path::PathBuf, sync::Arc};
pub use watching::{Observation as WatchObservation, Request as WatchRequest, Scope as WatchScope};
pub use work::{Completed, Work};
pub(crate) use work::{Finish, Finished};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Attention,
    Unsaved,
}
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub filter: Filter,
    pub page: usize,
}
pub struct Row {
    pub id: PoEntryId,
    pub source: String,
    pub caption: String,
    pub status: String,
    pub translation: String,
}
#[derive(Default)]
pub struct Queue {
    pub rows: Vec<Row>,
    pub total: usize,
    pub page: usize,
}
#[derive(Clone, PartialEq, Eq)]
pub struct EntryKey {
    pub catalogue: Identity,
    pub id: PoEntryId,
}
pub struct Entry {
    pub key: EntryKey,
    pub source: Vec<String>,
    pub context: Vec<String>,
    pub source_page: usize,
    pub context_page: usize,
    pub form_page: usize,
    pub details: Arc<EntryDetails>,
    pub wording_page: usize,
    pub rule: PluralRuleView,
    pub source_location: Option<String>,
}
pub struct PluralRuleView {
    pub pages: Vec<String>,
    pub page: usize,
    pub expanded: bool,
}

#[derive(Default)]
pub enum Destination {
    #[default]
    Queue,
    Entry,
    Comparison,
    Create,
    Refresh,
    Passage,
}
pub struct Comparison {
    pub inspection: Inspection,
    pub changes: Vec<ComparisonRow>,
    pub page: usize,
}
#[derive(Default)]
pub struct Localisation {
    pub slot: Slot,
    pub path: String,
    pub files_open: bool,
    pub query: Query,
    pub queue: Queue,
    pub destination: Destination,
    pub entry: Option<Entry>,
    pub comparison: Option<Comparison>,
    pub report: Vec<String>,
    pub report_page: usize,
    after_save: work::AfterSave,
    /// The queue's filter text changed while the catalogue was checked out; the query runs again
    /// when its owner returns.
    query_pending: bool,
    pub creation: creation::Creation,
    pub refresh: Option<refresh::Refresh>,
    pub passage: Option<beat::Beat>,
}
impl Localisation {
    pub fn preview_catalogue(&self) -> Option<&Catalogue> {
        self.slot.owner().map(|owner| &owner.catalogue)
    }
    pub fn preview_blocked(&self) -> bool {
        self.slot.read_only()
    }
    pub fn keep_editing(&mut self) {
        if let work::AfterSave::Sources { after, .. } = &mut self.after_save
            && matches!(after, SourceAfterSave::Close)
        {
            *after = SourceAfterSave::Stay;
        }
    }
    pub fn dirty(&self) -> bool {
        self.slot.dirty()
    }
    pub fn working(&self) -> bool {
        self.slot.working()
    }
    pub(crate) fn can_leave(&self) -> bool {
        !self.slot.read_only() && !self.needs_save() && !self.saving()
    }
    pub fn has_catalogue(&self) -> bool {
        self.slot.path().is_some()
    }
    pub fn needs_save(&self) -> bool {
        self.slot.needs_save()
    }
    pub fn saving(&self) -> bool {
        self.slot
            .owner()
            .is_some_and(|owner| owner.catalogue.saving)
    }
}
