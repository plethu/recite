//! Successful writes are harvested before the fixed recovery fence releases close and replacement.
use super::*;
use crate::editor_work::AfterSave as SourceAfterSave;
use recite_writer_session::{
    localisation::catalogue::{CatalogueDurability, CatalogueSave, CatalogueSaved},
    project::save::ordered::Scope,
};

#[derive(Default)]
pub(in crate::localisation) enum AfterSave {
    #[default]
    Stay,
    Sources {
        scope: Scope,
        after: SourceAfterSave,
    },
}
pub(super) struct Save {
    identity: Identity,
    request: Box<CatalogueSave>,
}
pub(super) struct Saved {
    identity: Identity,
    request: Box<CatalogueSave>,
    result: Option<Result<CatalogueSaved, String>>,
}
pub(crate) struct Finish {
    identity: Identity,
    ticket: CatalogueDurability,
}
pub(crate) struct Finished {
    identity: Identity,
    result: Result<(), String>,
}
impl Save {
    pub fn not_started(self, error: String) -> Saved {
        Saved {
            identity: self.identity,
            request: self.request,
            result: Some(Err(error)),
        }
    }
    pub fn execute(self) -> Saved {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.request.execute()))
                .unwrap_or_else(|_| {
                    Err(
                        "Catalogue save stopped unexpectedly. Inspect the PO file before retrying."
                            .into(),
                    )
                });
        Saved {
            identity: self.identity,
            request: self.request,
            result: Some(result),
        }
    }
}
impl Finish {
    pub(crate) fn execute(self) -> Finished {
        Finished {
            identity: self.identity,
            result: self.ticket.wait().map_err(|error| error.to_string()),
        }
    }
}
impl Writer {
    pub(crate) fn save_translation_drafts(&mut self) {
        self.begin_catalogue_save(None, AfterSave::Stay);
    }
    pub(crate) fn save_translation_entry(&mut self) {
        if let Some(entry) = &self.localisation.entry {
            let key = entry.key.clone();
            self.save_translation_key(&key);
        }
    }
    pub(in crate::localisation) fn save_translation_key(&mut self, key: &EntryKey) {
        if self.translation_entry_active(key) {
            self.begin_catalogue_save(Some(vec![key.id]), AfterSave::Stay);
        }
    }
    pub(crate) fn save_catalogue_then_sources(&mut self, scope: Scope, after: SourceAfterSave) {
        self.begin_catalogue_save(None, AfterSave::Sources { scope, after });
    }
    fn begin_catalogue_save(&mut self, entries: Option<Vec<PoEntryId>>, after: AfterSave) {
        if self.phase == Phase::Closed
            || self.session.read_only()
            || self.navigation.busy()
            || self.work.busy()
            || self.project_jobs_busy()
        {
            return;
        }
        let Some(owner) = self.localisation.slot.owner_mut() else {
            self.catalogue_feedback("The catalogue is unavailable for saving. Finish its action, or close with recovery and reopen after an unexpected stop.".into());
            return;
        };
        match owner.catalogue.begin_save(entries.as_deref()) {
            Ok(request) => {
                let identity = owner.identity.clone();
                self.localisation.after_save = after;
                self.submit_catalogue_tool(Work {
                    kind: Kind::Saving(Save {
                        identity,
                        request: Box::new(request),
                    }),
                });
            }
            Err(error) => self.catalogue_feedback(error),
        }
    }
    pub(super) fn accept_catalogue_save(
        &mut self,
        saved: &mut Saved,
    ) -> Result<Option<Finish>, String> {
        let owner = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == saved.identity)
            .ok_or("The saved catalogue is no longer open.")?;
        let result = saved
            .result
            .take()
            .ok_or("The catalogue save was already harvested.")?
            .and_then(|receipt| owner.catalogue.accept_save(&mut saved.request, receipt));
        match result {
            Ok(Some(ticket)) => Ok(Some(Finish {
                identity: owner.identity.clone(),
                ticket,
            })),
            Ok(None) => {
                self.finish_catalogue_save(Ok(()));
                Ok(None)
            }
            Err(error) => {
                self.finish_catalogue_save(Err(error));
                Ok(None)
            }
        }
    }
    pub(crate) fn finish_catalogue_tool(&mut self, finished: &mut Finished) -> Result<(), String> {
        if self.localisation.slot.identity() != Some(&finished.identity) {
            return Err("The catalogue changed while its recovery write was finishing.".into());
        }
        self.finish_catalogue_save(std::mem::replace(&mut finished.result, Ok(())));
        Ok(())
    }
    fn finish_catalogue_save(&mut self, result: Result<(), String>) {
        if let Some(owner) = self.localisation.slot.owner_mut() {
            owner.catalogue.finish_save();
        }
        let after = std::mem::take(&mut self.localisation.after_save);
        match result {
            Ok(()) => {
                self.catalogue_feedback(if self.localisation.dirty() { "Saved translations; newer translation drafts remain open." } else { "Translations saved." }.into());
                if let AfterSave::Sources { scope, after } = after { self.resume_source_save(scope, after); }
            }
            Err(error) => self.catalogue_feedback(format!("Catalogue save or recovery is incomplete: {error}. Keep Writer open and retry Save.")),
        }
    }
}
