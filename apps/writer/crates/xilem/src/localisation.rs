//! Translation edits use the shared file-backed catalogue, with one native presentation owner.
mod slot;
#[cfg(test)]
mod tests;
mod view;
mod work;
use crate::{
    Phase, Writer,
    editor_work::AfterSave as SourceAfterSave,
    workspaces::{Screen, text},
};
use recite_core::po::PoEntryId;
use recite_writer_session::localisation::status::TranslationStatus;
use recite_writer_session::{
    comparison::ComparisonRow,
    localisation::catalogue::{ExternalDecision, Inspection},
};
use slot::{Identity, Slot};
use std::path::PathBuf;
pub(crate) use view::{close_details, panel};
pub(crate) use work::{Completed, Work};
pub(crate) use work::{Finish, Finished};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Attention,
    Unsaved,
}
#[derive(Clone, Default, PartialEq, Eq)]
struct Query {
    text: String,
    filter: Filter,
    page: usize,
}
struct Row {
    id: PoEntryId,
    source: String,
    caption: String,
    status: String,
}
#[derive(Default)]
struct Queue {
    rows: Vec<Row>,
    total: usize,
    page: usize,
}
#[derive(Clone, PartialEq)]
struct EntryKey {
    catalogue: Identity,
    id: PoEntryId,
}
struct Entry {
    key: EntryKey,
    source: Vec<String>,
    context: Vec<String>,
    source_page: usize,
    context_page: usize,
    form_page: usize,
}
#[derive(Default)]
enum Destination {
    #[default]
    Queue,
    Entry,
    Comparison,
}
struct Comparison {
    inspection: Inspection,
    changes: Vec<ComparisonRow>,
    page: usize,
}
#[derive(Default)]
pub(crate) struct Localisation {
    slot: Slot,
    path: String,
    files_open: bool,
    query: Query,
    queue: Queue,
    destination: Destination,
    entry: Option<Entry>,
    comparison: Option<Comparison>,
    report: Vec<String>,
    report_page: usize,
    after_save: work::AfterSave,
}
impl Localisation {
    pub(crate) fn keep_editing(&mut self) {
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
    pub(crate) fn open_localisation(&mut self) {
        if self.phase != Phase::Writing
            || self.session.project().is_none()
            || self.project_action_blocked()
        {
            return;
        }
        self.attention.leave_focus();
        self.cancel_map_gesture();
        self.workspaces.screen = Screen::Localisation;
    }
    pub(crate) fn catalogue_feedback(&mut self, message: String) {
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
    fn catalogue_action(&mut self, action: work::Action) {
        let closing = matches!(action, work::Action::FlushForClose);
        if self.phase == Phase::Closed
            || (!closing && (self.phase != Phase::Writing || self.localisation.slot.read_only()))
            || self.session.read_only()
            || self.navigation.busy()
            || self.work.busy()
            || self.project_jobs_busy()
        {
            return;
        }
        let selected = self.localisation.entry.as_ref().map(|entry| entry.key.id);
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
    pub(crate) fn open_catalogue_path(&mut self) {
        let path = PathBuf::from(self.localisation.path.trim());
        if !path.as_os_str().is_empty() {
            self.catalogue_action(work::Action::Open(path));
        }
    }
    pub(crate) fn flush_catalogue_before_close(&mut self) {
        self.catalogue_action(work::Action::FlushForClose);
    }
    pub(crate) fn search_catalogue(&mut self) {
        self.localisation.query.page = 0;
        self.catalogue_action(work::Action::Query);
    }
    fn inspect_translation(&mut self, key: &EntryKey) {
        if self.localisation.slot.identity() == Some(&key.catalogue) {
            self.catalogue_action(work::Action::Inspect(key.id));
        }
    }
    fn update_translation(&mut self, key: &EntryKey, arm: usize, text: String) {
        if self.phase != Phase::Writing {
            return;
        }
        let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
        else {
            return;
        };
        let Some(mut draft) = owner.catalogue.draft(key.id) else {
            return;
        };
        let Some(form) = draft.forms.get_mut(arm) else {
            return;
        };
        *form = text;
        draft.reviewed = false;
        owner.catalogue.update(key.id, draft);
        self.status = "Translation draft updated; Save publishes the PO file.".into();
    }
    fn review_translation(&mut self, key: &EntryKey, reviewed: bool) {
        if self.phase != Phase::Writing {
            return;
        }
        if let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
            && let Some(mut draft) = owner.catalogue.draft(key.id)
        {
            draft.reviewed = reviewed;
            owner.catalogue.update(key.id, draft);
        }
    }
    fn discard_translation(&mut self, key: &EntryKey) {
        if self.phase != Phase::Writing {
            return;
        }
        if let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
        {
            owner.catalogue.discard(key.id);
        }
    }
    pub(crate) fn cancel_catalogue_action(&mut self) {
        self.localisation.slot.cancel();
        self.status = "Cancellation requested; waiting for the catalogue owner to return.".into();
    }
    fn compare_catalogue(&mut self) {
        self.catalogue_action(work::Action::Compare);
    }
    fn resolve_catalogue(&mut self, decision: ExternalDecision) {
        if let Some(comparison) = &self.localisation.comparison {
            self.catalogue_action(work::Action::Resolve {
                decision,
                inspection: Box::new(comparison.inspection.clone()),
            });
        }
    }
}
