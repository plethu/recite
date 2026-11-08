//! Translation edits use the shared file-backed catalogue, with one native presentation owner.
pub mod beat;
pub mod closing;
pub mod creation;
pub mod editing;
pub mod history;
pub mod keyboard;
pub mod location;
pub mod presentation;
pub mod refresh;
pub mod slot;
pub mod source_navigation;
#[cfg(any(test, feature = "test-support"))]
pub mod tests;
pub mod watching;
pub mod work;
use crate::{
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
use slot::{Identity, Slot};
pub use source_navigation::SourceIntent;
use std::{path::PathBuf, sync::Arc};
pub use watching::{Observation as WatchObservation, Request as WatchRequest, Scope as WatchScope};
pub use work::{Completed, Work};
pub use work::{Finish, Finished};

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
    pub fn can_leave(&self) -> bool {
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
impl Writer {
    pub fn open_localisation(&mut self) {
        if !self.can_open_localisation() {
            return;
        }
        if self.localisation.has_catalogue() {
            self.open_translation_beat(0);
        } else {
            self.enter_localisation();
        }
    }
    fn can_open_localisation(&self) -> bool {
        self.phase == Phase::Writing
            && self.session.project().is_some()
            && !self.project_action_blocked()
    }
    pub fn enter_localisation(&mut self) -> bool {
        if !self.can_open_localisation() {
            return false;
        }
        self.attention.leave_focus();
        self.cancel_map_gesture();
        self.remember_location();
        self.workspaces.screen = Screen::Localisation;
        self.remember_catalogue_location();
        true
    }
    pub fn catalogue_feedback(&mut self, message: String) {
        self.status = message
            .lines()
            .next()
            .unwrap_or("Catalogue action finished")
            .chars()
            .take(240)
            .collect();
        self.localisation.report = if message.contains('\n') || message.chars().count() > 240 {
            text::pages(&message)
        } else {
            Vec::new()
        };
        self.localisation.report_page = 0;
    }
    pub fn catalogue_action(&mut self, action: work::Action) {
        let closing = matches!(
            action,
            work::Action::FlushForClose | work::Action::DiscardForClose
        );
        if self.phase == Phase::Closed
            || (!closing && (self.phase != Phase::Writing || self.localisation.slot.read_only()))
            || (self.session.read_only() && !closing)
            || self.navigation.busy()
            || self.work.busy()
            || self.project_jobs_busy()
        {
            return;
        }
        let selected = self.localisation.mounted_entries();
        match self.localisation.slot.checkout(selected) {
            Ok(checkout) => self.submit_catalogue_tool(Work::checked(
                checkout,
                action,
                self.localisation.query.clone(),
                self.session.model().document().cached_passages(),
            )),
            Err(error) => self.catalogue_feedback(error),
        }
    }
    pub fn open_catalogue_path(&mut self) {
        let path = PathBuf::from(self.localisation.path.trim());
        if !path.as_os_str().is_empty() {
            self.catalogue_action(work::Action::Open(path));
        }
    }
    pub fn flush_catalogue_before_close(&mut self) {
        self.catalogue_action(work::Action::FlushForClose);
    }
    pub fn search_catalogue(&mut self) {
        self.localisation.query.page = 0;
        self.catalogue_action(work::Action::Query);
    }
    pub fn inspect_translation(&mut self, key: &EntryKey) {
        if self.localisation.slot.identity() == Some(&key.catalogue) {
            self.catalogue_action(work::Action::Inspect(key.id));
        }
    }
    pub fn cancel_catalogue_action(&mut self) {
        self.localisation.slot.cancel();
        self.session.cancel_exclusive();
        self.status = "Cancellation requested; waiting for the catalogue owner to return.".into();
    }
    pub fn compare_catalogue(&mut self) {
        self.catalogue_action(work::Action::Compare);
    }
    pub fn resolve_catalogue(&mut self, decision: ExternalDecision) {
        if let Some(comparison) = &self.localisation.comparison {
            self.catalogue_action(work::Action::Resolve {
                decision,
                inspection: Box::new(comparison.inspection.clone()),
            });
        }
    }
}
