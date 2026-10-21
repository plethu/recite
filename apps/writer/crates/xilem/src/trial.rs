//! One in-flight compilation; the worker also destroys rejected/replaced trials.
use super::{Phase, Writer};
use crate::{
    attention::{Pane, PaneMotion},
    map::Workspace,
    workspaces::Screen,
};
use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{PreparedPreview, Preview};
use recite_writer_session::preview::{Request, trace::Browser};
use std::{
    fmt,
    sync::{Arc, mpsc},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

mod inputs;
pub(super) use inputs::Settings;

pub(super) struct Trial {
    sender: Option<UnboundedSender<Arc<Request>>>,
    preparation: Preparation,
    pub settings: Settings,
    pub visible: bool,
    pub answer: String,
    pub status: String,
    pub choice_page: usize,
    pub effect_page: usize,
    pub trace: Browser,
}

enum Preparation {
    Idle,
    Running(Arc<Request>),
    Cancelling,
}

enum Retired {
    Prepared(Box<PreparedPreview>),
    Preview(Box<Preview>),
}

struct Output {
    request: Arc<Request>,
    result: Result<Box<PreparedPreview>, String>,
    receipt: mpsc::SyncSender<(Arc<Request>, Option<Retired>)>,
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
            preparation: Preparation::Idle,
            settings: Settings::default(),
            visible: false,
            answer: String::new(),
            status: "Effects are displayed, never executed. Restart captures your staged inputs."
                .into(),
            choice_page: 0,
            effect_page: 0,
            trace: Default::default(),
        }
    }

    pub fn busy(&self) -> bool {
        !matches!(self.preparation, Preparation::Idle)
    }

    pub fn cancel(&mut self) {
        if self.busy() {
            self.preparation = Preparation::Cancelling;
            self.status = "Cancelled; finishing the current compiler phase.".into();
        }
    }
}

impl Writer {
    pub(super) fn start_trial(&mut self) {
        if self.session.read_only() || self.localisation.preview_blocked() {
            self.trial.status = "Finish the project action before starting a preview.".into();
            return;
        }
        self.workspaces.screen = Screen::Write;
        self.attention.leave_focus();
        self.trial.visible = true;
        self.focus_pane(PaneMotion::To(Pane::Preview));
        if self.trial.busy() {
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
            Ok(())
        });
        self.trial.status = match result {
            Ok(()) => "Preparing preview… You can keep editing.".into(),
            Err(error) => error,
        };
    }
    pub(super) fn configure_trial(&mut self) {
        self.attention.leave_focus();
        self.workspaces.screen = Screen::Write;
        self.trial.visible = true;
        self.trial.settings.open = true;
        self.document_menu = crate::attention::DocumentMenu::Closed;
        self.focus_pane(PaneMotion::To(Pane::Preview));
    }
    pub(super) fn configuring_trial(&self) -> bool {
        self.trial.visible && self.trial.settings.open && !self.attention.is_focused()
    }
    pub(super) fn close_trial_inputs(&mut self) {
        self.trial.settings.open = false;
        self.focus_pane(PaneMotion::To(Pane::Preview));
    }
    pub(super) fn return_from_trial(&mut self) {
        self.trial.settings.open = false;
        self.trial.visible = false;
        let pane = if self.visible_workspace() == Workspace::Map {
            Pane::Map
        } else {
            Pane::Script
        };
        self.focus_pane(PaneMotion::To(pane));
    }

    fn accept_trial(&mut self, output: Output) {
        let preparation = std::mem::replace(&mut self.trial.preparation, Preparation::Idle);
        let current = matches!(preparation, Preparation::Running(ref request) if Arc::ptr_eq(request, &output.request))
            && self.phase == Phase::Writing
            && output
                .request
                .is_current(self.session.model(), self.localisation.preview_catalogue());
        drop(preparation);
        if !current {
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
                    self.trial.status = "Preview uses the accepted source snapshot.".into();
                    previous.map(|preview| Retired::Preview(Box::new(preview)))
                }
                Err(prepared) => Some(Retired::Prepared(prepared)),
            },
            Ok(prepared) => Some(Retired::Prepared(prepared)),
            Err(error) => {
                if current {
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

pub(super) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |state: &mut Writer, sender| state.trial.sender = Some(sender),
        Writer::accept_trial,
    )
}

async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Arc<Request>>) {
    // The UI admits one request until its output is acknowledged, including after Cancel.
    // This bounds the toolkit's unbounded channel without creating a second task system.
    while let Some(request) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                request.prepare(&CancellationToken::new())
            }))
            .unwrap_or_else(|_| {
                Err("Preview preparation stopped unexpectedly. Restart to retry.".into())
            });
            let (receipt, returned) = mpsc::sync_channel(1);
            if proxy
                .message(Output {
                    request,
                    result,
                    receipt,
                })
                .is_ok()
            {
                let retired = returned.recv().ok().and_then(|(request, retired)| {
                    drop(request);
                    retired
                });
                match retired {
                    Some(Retired::Prepared(prepared)) => drop(prepared),
                    Some(Retired::Preview(preview)) => drop(preview),
                    None => {}
                }
            }
        })
        .await;
    }
}

#[cfg(test)]
mod tests;
