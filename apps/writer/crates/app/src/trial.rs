//! One in-flight compilation; the worker also destroys rejected/replaced trials.
use super::{Phase, Writer};
use crate::{
    attention::{Pane, PaneMotion},
    map::Workspace,
    workspaces::Screen,
};
use recite_writer_model::{PreparedPreview, Preview};
use recite_writer_session::preview::{Request, trace::Browser};
use std::{
    fmt,
    sync::{Arc, mpsc},
};
use tokio::sync::mpsc::UnboundedSender;

pub mod inputs;
pub use inputs::{Review, Settings, VALUE_PAGE};
pub mod preparation;
pub mod recovery;
use preparation::{Outcome, Preparation};

pub struct Trial {
    pub sender: Option<UnboundedSender<Arc<Request>>>,
    pub preparation: Preparation,
    pub settings: Settings,
    pub visible: bool,
    pub answer: String,
    pub status: String,
    pub choice_page: usize,
    pub effect_page: usize,
    pub trace: Browser,
    pub context_details: bool,
}

/// A preview or preparation the worker drops off the UI thread.
pub enum Retired {
    Prepared(Box<PreparedPreview>),
    Preview(Box<Preview>),
}

/// A finished preparation and the channel that returns whatever the UI retires.
pub struct Output {
    pub request: Arc<Request>,
    pub result: Result<Box<PreparedPreview>, String>,
    pub receipt: mpsc::SyncSender<(Arc<Request>, Option<Retired>)>,
}

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewOutput")
            .field("successful", &self.result.is_ok())
            .finish_non_exhaustive()
    }
}

impl Trial {
    pub fn new() -> Self {
        Self {
            sender: None,
            preparation: Preparation::Idle(Outcome::Ready),
            settings: Settings::default(),
            visible: false,
            answer: String::new(),
            status: preparation::GUIDANCE.into(),
            choice_page: 0,
            effect_page: 0,
            trace: Default::default(),
            context_details: false,
        }
    }
}

impl Writer {
    pub fn start_trial(&mut self) {
        if let Err(error) = self.require_finished_composition() {
            self.status = error;
            return;
        }
        if self.session.read_only() || self.localisation.preview_blocked() {
            self.trial.status = "Finish the project action before starting a preview.".into();
            return;
        }
        self.workspaces.screen = Screen::Write;
        self.attention.leave_focus();
        self.trial.visible = true;
        self.trial.settings.review = Review::Inputs;
        self.focus_pane(PaneMotion::To(Pane::Preview));
        if self.trial.busy() || self.navigation.busy() {
            return;
        }
        if self.session.model().has_draft()
            && let Some(target) = crate::attention::EditingTarget::capture(self)
        {
            self.trial.preparation = Preparation::Idle(Outcome::UnfinishedEdit(target));
            self.trial.settings.open = false;
            self.trial.status = "Your unfinished manuscript edit is kept. Apply it to include it in the preview, or return to writing.".into();
            return;
        }
        let result = Request::capture(
            self.session.model(),
            self.trial.settings.capture(),
            self.localisation.preview_catalogue(),
        )
        .and_then(|request| {
            let request = Arc::new(request);
            self.trial
                .sender
                .as_ref()
                .ok_or("Preview worker is unavailable. Reopen Writer to retry.".to_owned())?
                .send(request.clone())
                .map_err(|_| "Preview worker stopped. Reopen Writer to retry.".to_owned())?;
            self.trial.preparation = Preparation::Running(request);
            self.trial.settings.open = false;
            Ok(())
        });
        self.trial.status = match result {
            Ok(()) => "Preparing preview… You can keep editing.".into(),
            Err(error) => {
                self.trial.preparation = Preparation::Idle(Outcome::Failed(error.clone()));
                error
            }
        };
    }
    pub fn configure_trial(&mut self) {
        if let Err(error) = self.require_finished_composition() {
            self.status = error;
            return;
        }
        self.attention.leave_focus();
        self.workspaces.screen = Screen::Write;
        self.trial.visible = true;
        self.trial.settings.open = true;
        self.trial.settings.review = Review::Inputs;
        self.document_menu = crate::attention::DocumentMenu::Closed;
        self.focus_pane(PaneMotion::To(Pane::Preview));
    }
    pub fn configuring_trial(&self) -> bool {
        self.trial.visible && self.trial.settings.open && !self.attention.is_focused()
    }
    pub fn close_trial_inputs(&mut self) {
        self.trial.settings.open = false;
        self.trial.settings.review = Review::Inputs;
        self.focus_pane(PaneMotion::To(Pane::Preview));
    }
    pub fn return_from_trial(&mut self) {
        if self.navigation.applying_preview() {
            self.cancel_navigation();
        }
        self.trial.settings.open = false;
        self.trial.settings.review = Review::Inputs;
        self.trial.visible = false;
        let pane = if self.visible_workspace() == Workspace::Map {
            Pane::Map
        } else {
            Pane::Script
        };
        self.focus_pane(PaneMotion::To(pane));
    }

    pub fn accept_trial(&mut self, output: Output) {
        let preparation = std::mem::replace(
            &mut self.trial.preparation,
            Preparation::Idle(Outcome::Ready),
        );
        let current = matches!(preparation, Preparation::Running(ref request) if Arc::ptr_eq(request, &output.request))
            && self.phase == Phase::Writing
            && output
                .request
                .is_current(self.session.model(), self.localisation.preview_catalogue());
        let cancelled = matches!(preparation, Preparation::Cancelling);
        drop(preparation);
        if !current {
            self.trial.preparation = Preparation::Idle(if cancelled {
                Outcome::Cancelled
            } else {
                Outcome::Changed
            });
            self.trial.status =
                "Preview inputs changed or preparation was cancelled. Restart when ready.".into();
        }
        let retired = match output.result {
            Ok(prepared) if current => match self.session.accept_preview(prepared) {
                Ok(previous) => {
                    self.trial.answer.clear();
                    self.trial.choice_page = 0;
                    self.trial.effect_page = 0;
                    self.trial.trace.restart();
                    self.trial.context_details = false;
                    self.trial.status.clear();
                    previous.map(|preview| Retired::Preview(Box::new(preview)))
                }
                Err(prepared) => {
                    self.trial.preparation = Preparation::Idle(Outcome::Changed);
                    self.trial.status =
                        "Preview inputs changed before publication. Restart when ready.".into();
                    Some(Retired::Prepared(prepared))
                }
            },
            Ok(prepared) => Some(Retired::Prepared(prepared)),
            Err(error) => {
                if current {
                    self.trial.preparation = Preparation::Idle(Outcome::Failed(error.clone()));
                    self.trial.status = if self.session.model().preview_page().is_some() {
                        format!("{error}\nThe running preview below is unchanged.")
                    } else {
                        error
                    };
                }
                None
            }
        };
        // The receiver stays alive until this receipt or the window's queued output drops.
        let _ = output.receipt.send((output.request, retired));
    }
}

impl Default for Trial {
    fn default() -> Self {
        Self::new()
    }
}
