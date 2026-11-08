//! Source actions hold both real owners until their terminal outcome returns.
pub mod accepting;
#[cfg(test)]
mod tests;
use super::*;
use crate::localisation::presentation;
use crate::session::{Checkout as ProjectCheckout, RecoveryReason};
use recite_compiler::authoring::WorkControl;
use recite_writer_session::{
    ProjectSession,
    localisation::{
        catalogue::RefreshPreview,
        create, extraction,
        messages::{MsgId, text as wording},
        target::TargetLocale,
    },
};
use slot::{Checkout, Owner};

pub enum Action {
    Create(TargetLocale),
    Review,
    Apply(Arc<RefreshPreview>),
}
pub struct Work {
    project: ProjectCheckout,
    catalogue: Checkout,
    action: Action,
    query: Query,
}
pub struct Completed {
    project: ProjectCheckout,
    catalogue: Checkout,
    result: Result<Outcome, String>,
    previous: Option<Identity>,
    recovery: Option<RecoveryReason>,
    retired_entry: Option<Entry>,
    retired_comparison: Option<Comparison>,
    retired_refresh: Option<refresh::Refresh>,
    retired_beat: Option<beat::Beat>,
}
pub enum Outcome {
    Queue { queue: Queue, message: String },
    Review(Option<refresh::Refresh>),
}
impl Work {
    fn complete(
        self,
        result: Result<Outcome, String>,
        previous: Option<Identity>,
        recovery: Option<RecoveryReason>,
    ) -> Completed {
        Completed {
            project: self.project,
            catalogue: self.catalogue,
            result,
            previous,
            recovery,
            retired_entry: None,
            retired_comparison: None,
            retired_refresh: None,
            retired_beat: None,
        }
    }
    pub fn not_started(self, error: String) -> Completed {
        let previous = self
            .catalogue
            .owner
            .as_ref()
            .map(|owner| owner.identity.clone());
        self.complete(Err(error), previous, None)
    }
    pub fn execute(self) -> Completed {
        self.execute_with(perform)
    }
    fn execute_with(
        mut self,
        perform: impl FnOnce(&ProjectSession, &mut Checkout, &Action, &Query) -> Result<Outcome, String>,
    ) -> Completed {
        let previous = self
            .catalogue
            .owner
            .as_ref()
            .map(|owner| owner.identity.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.catalogue
                .ticket
                .checkpoint()
                .map_err(|e| e.to_string())?;
            let project = self
                .project
                .project
                .as_mut()
                .ok_or("The project owner is unavailable.")?;
            project.checkpoint().map_err(|e| e.to_string())?;
            if let Some(owner) = self.catalogue.owner.as_mut() {
                owner.catalogue.require_idle_save()?;
                if owner.catalogue.dirty() {
                    return Err(wording(MsgId::WriterReloadDrafts));
                }
                owner.flush_recovery()?;
            }
            perform(project, &mut self.catalogue, &self.action, &self.query)
        }));
        let (result, recovery) = match result {
            Ok(result) => (result, None),
            Err(_) => {
                if let Some(owner) = self.catalogue.owner.as_mut() {
                    owner.quarantine();
                }
                (Err("Catalogue source operation stopped unexpectedly. Close with recovery and reopen before editing.".into()), Some(RecoveryReason::ReopenRequired))
            }
        };
        self.complete(result, previous, recovery)
    }
}
fn perform(
    project: &ProjectSession,
    checkout: &mut Checkout,
    action: &Action,
    query: &Query,
) -> Result<Outcome, String> {
    let control = checkout.ticket.clone();
    let template =
        extraction::template(project.current().model(), Some(project.files()), &control)?;
    let message = match action {
        Action::Create(locale) => {
            let path = create::suggested_path(project.root(), &locale.to_string());
            if path.try_exists().map_err(|e| e.to_string())? {
                return Err(wording(MsgId::WriterCatalogueExists));
            }
            let document = create::prepare(&template, locale, &control)?;
            let current =
                extraction::template(project.current().model(), Some(project.files()), &control)?;
            if current != template {
                return Err(wording(MsgId::WriterCreationChanged));
            }
            control.checkpoint().map_err(|e| e.to_string())?;
            checkout.owner = Some(Box::new(Owner::new(create::persist(&document, &path)?)));
            "Catalogue created. Translations start blank; source files stay unchanged.".into()
        }
        Action::Review => {
            let owner = checkout.owner.as_ref().ok_or("Open a catalogue first.")?;
            return Ok(Outcome::Review(Some(refresh::Refresh::new(
                RefreshPreview::prepare(&owner.catalogue, template)?,
            ))));
        }
        Action::Apply(preview) => {
            let owner = checkout.owner.as_mut().ok_or("Open a catalogue first.")?;
            let before = owner.catalogue.document.clone();
            let result = preview.apply(&mut owner.catalogue, &template);
            if !Arc::ptr_eq(&before, &owner.catalogue.document) {
                owner.renew_identity();
            }
            let receipt = result?;
            receipt.recovery_error.map_or_else(
                || "Source updates saved to the PO file. Changed translations need review.".into(),
                |error| format!("Source updates saved to the PO file; recovery is incomplete.\n{error}\nRetry Save or keep this window open."),
            )
        }
    };
    let owner = checkout
        .owner
        .as_ref()
        .ok_or("The catalogue owner is unavailable.")?;
    let presentation = presentation::describe(
        owner,
        query,
        None,
        project.current().model().document().cached_passages(),
    );
    match presentation {
        Ok(presentation) => Ok(Outcome::Queue {
            queue: presentation.queue,
            message,
        }),
        Err(error) => Ok(Outcome::Queue {
            queue: Queue::default(),
            message: format!(
                "{message}\nThe translation queue could not be prepared: {error}. Search the queue again to retry."
            ),
        }),
    }
}
