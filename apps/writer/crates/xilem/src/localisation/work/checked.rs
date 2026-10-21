//! Disk work owns the real catalogue and always returns it, including failure and cancellation.
use super::*;
use crate::localisation::presentation::{Presentation, describe};
use recite_compiler::authoring::WorkControl;
use recite_writer_model::Passage;
use recite_writer_session::localisation::catalogue::Catalogue;
use slot::{Checkout, Owner};
use std::sync::Arc;

pub(in crate::localisation) enum Action {
    Open(PathBuf),
    Query,
    Inspect(PoEntryId),
    Reload,
    Close,
    FlushForClose,
    Compare,
    Resolve {
        decision: ExternalDecision,
        inspection: Box<Inspection>,
    },
}
pub(super) struct Checked {
    checkout: Checkout,
    action: Action,
    query: Query,
    passages: Option<Arc<[Passage]>>,
    previous: Option<Identity>,
}
pub(super) struct CheckedCompleted {
    checkout: Checkout,
    result: Result<Completion, String>,
    previous: Option<Identity>,
    retired_entry: Option<Entry>,
    retired_comparison: Option<Comparison>,
    retired_refresh: Option<refresh::Refresh>,
}
enum Completion {
    Show(Presentation),
    Compared(Option<Comparison>),
    CloseWindow,
    ClosedCatalogue,
}

impl Checked {
    pub(super) fn new(
        checkout: Checkout,
        action: Action,
        query: Query,
        passages: Option<Arc<[Passage]>>,
    ) -> Self {
        Self {
            previous: checkout.owner.as_ref().map(|owner| owner.identity.clone()),
            checkout,
            action,
            query,
            passages,
        }
    }
    pub(super) fn not_started(self, error: String) -> CheckedCompleted {
        CheckedCompleted {
            previous: self.previous,
            retired_entry: None,
            retired_comparison: None,
            retired_refresh: None,
            checkout: self.checkout,
            result: Err(error),
        }
    }
    pub(super) fn execute(self) -> CheckedCompleted {
        self.execute_with(perform)
    }
    fn execute_with(
        self,
        perform: impl FnOnce(&mut Checkout, &Action) -> Result<(), String>,
    ) -> CheckedCompleted {
        let Self {
            mut checkout,
            action,
            query,
            passages,
            previous,
        } = self;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            checkout
                .ticket
                .checkpoint()
                .map_err(|error| error.to_string())?;
            // Establish a durable admitted snapshot before any exclusive mutation.
            // A quarantined owner retries only that snapshot, never uncertain fields.
            if let Some(owner) = checkout.owner.as_mut() {
                owner.flush_recovery()?;
            }
            perform(&mut checkout, &action)?;
            if matches!(action, Action::FlushForClose) {
                return Ok(Completion::CloseWindow);
            }
            if matches!(action, Action::Close) {
                return Ok(Completion::ClosedCatalogue);
            }
            let owner = checkout.owner.as_ref().ok_or("Open a catalogue first.")?;
            if matches!(action, Action::Compare) {
                let compared = owner.catalogue.compare()?;
                let changes = compared
                    .rows
                    .into_iter()
                    .flat_map(|mut row| {
                        row.caption = row.caption.chars().take(160).collect();
                        text::change_pages(row)
                    })
                    .collect();
                return Ok(Completion::Compared(Some(Comparison {
                    inspection: compared.inspection,
                    changes,
                    page: 0,
                })));
            }
            describe(
                owner,
                &query,
                match action {
                    Action::Inspect(id) => Some(id),
                    _ => None,
                },
                passages,
            )
            .map(Completion::Show)
        }));
        let result = result.unwrap_or_else(|_| {
            if let Some(owner) = checkout.owner.as_mut() { owner.quarantine(); }
            Err("Catalogue operation stopped unexpectedly. Close with recovery and reopen before editing translations.".into())
        });
        CheckedCompleted {
            checkout,
            result,
            previous,
            retired_entry: None,
            retired_comparison: None,
            retired_refresh: None,
        }
    }
}
fn perform(checkout: &mut Checkout, action: &Action) -> Result<(), String> {
    match action {
        Action::Open(path) => {
            if let Some(owner) = checkout.owner.as_mut() {
                if owner.catalogue.dirty() {
                    return Err(
                        "Save or discard translation drafts before opening another catalogue."
                            .into(),
                    );
                }
                owner.flush_recovery()?;
            }
            let catalogue = Catalogue::open_recoverable(path)?;
            checkout
                .ticket
                .checkpoint()
                .map_err(|error| error.to_string())?;
            checkout.owner = Some(Box::new(Owner::new(catalogue)));
        }
        Action::Reload => {
            replace_catalogue(checkout, Catalogue::reload)?;
        }
        Action::Close | Action::FlushForClose => {
            if let Some(owner) = checkout.owner.as_mut() {
                if matches!(action, Action::Close) && owner.catalogue.dirty() {
                    return Err(
                        "Save or discard translation drafts before closing the catalogue.".into(),
                    );
                }
                owner.flush_recovery()?;
            }
            if matches!(action, Action::Close) {
                checkout.owner = None;
            }
        }
        Action::Resolve {
            decision,
            inspection,
        } => {
            replace_catalogue(checkout, |catalogue| {
                catalogue.accept_external(*decision, inspection)
            })?;
        }
        Action::Query | Action::Inspect(_) | Action::Compare => (),
    }
    Ok(())
}
fn replace_catalogue(
    checkout: &mut Checkout,
    replace: impl FnOnce(&mut Catalogue) -> Result<(), String>,
) -> Result<(), String> {
    let owner = checkout.owner.as_mut().ok_or("Open a catalogue first.")?;
    let before = owner.catalogue.document.clone();
    let result = replace(&mut owner.catalogue);
    if !Arc::ptr_eq(&before, &owner.catalogue.document) {
        owner.renew_identity();
    }
    result
}
impl Writer {
    pub(super) fn accept_checked_catalogue(
        &mut self,
        completed: &mut CheckedCompleted,
    ) -> Result<(), String> {
        self.localisation.slot.restore(&mut completed.checkout)?;
        if self.localisation.slot.identity() != completed.previous.as_ref() {
            completed.retired_entry = self.localisation.entry.take();
            completed.retired_comparison = self.localisation.comparison.take();
            completed.retired_refresh = self.localisation.refresh.take();
            self.localisation.queue = Queue::default();
            self.localisation.destination = Destination::Queue;
            self.localisation.files_open = false;
        }
        match &mut completed.result {
            Ok(Completion::Compared(comparison)) => {
                let incoming = comparison
                    .take()
                    .ok_or("The catalogue comparison was already accepted.")?;
                completed.retired_comparison = self.localisation.comparison.replace(incoming);
                self.localisation.destination = Destination::Comparison;
                self.catalogue_feedback(
                    "Catalogue comparison ready. Choose which translation version to keep.".into(),
                );
            }
            Ok(Completion::Show(presentation)) => {
                self.localisation.queue = std::mem::take(&mut presentation.queue);
                self.localisation.query.page = self.localisation.queue.page;
                std::mem::swap(&mut self.localisation.entry, &mut presentation.entry);
                self.localisation.destination = if self.localisation.entry.is_some() {
                    Destination::Entry
                } else {
                    Destination::Queue
                };
                self.catalogue_feedback(presentation.message.into());
            }
            Err(error) => {
                if self.phase == Phase::Closing {
                    self.phase = Phase::ConfirmClose;
                }
                self.catalogue_feedback(error.clone());
            }
            Ok(Completion::CloseWindow) if self.phase == Phase::Closing => {
                self.close_sources_with_recovery()
            }
            Ok(Completion::CloseWindow) => {
                self.catalogue_feedback("Recovery completed. Writer stays open.".into())
            }
            Ok(Completion::ClosedCatalogue) => {
                completed.retired_entry = self
                    .localisation
                    .entry
                    .take()
                    .or_else(|| completed.retired_entry.take());
                self.localisation.queue = Queue::default();
                self.localisation.destination = Destination::Queue;
                self.catalogue_feedback("Catalogue closed after recovery became durable.".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
