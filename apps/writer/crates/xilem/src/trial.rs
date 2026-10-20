//! One in-flight compilation; the worker also destroys rejected/replaced trials.
use super::{Phase, Writer};
use recite_writer_model::{PreparedPreview, Preview, PreviewRequest, PreviewSetup};
use std::{
    fmt,
    sync::{Arc, mpsc},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

pub(super) struct Trial {
    sender: Option<UnboundedSender<Arc<PreviewRequest>>>,
    preparation: Preparation,
    pub visible: bool,
    pub answer: String,
    pub status: String,
    pub choice_page: usize,
    pub effect_page: usize,
}

enum Preparation {
    Idle,
    Running(Arc<PreviewRequest>),
    Cancelling,
}

enum Retired {
    Prepared(Box<PreparedPreview>),
    Preview(Box<Preview>),
}

struct Output {
    request: Arc<PreviewRequest>,
    result: Result<Box<PreparedPreview>, String>,
    receipt: mpsc::SyncSender<Option<Retired>>,
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
            visible: false,
            answer: String::new(),
            status: "Source-language playback. Effects are displayed, never executed.".into(),
            choice_page: 0,
            effect_page: 0,
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
        self.trial.visible = true;
        if self.trial.busy() {
            return;
        }
        let result = self
            .session
            .model()
            .preview_request()
            .map_err(|error| error.to_string())
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

    fn accept_trial(&mut self, output: Output) {
        let preparation = std::mem::replace(&mut self.trial.preparation, Preparation::Idle);
        let current = matches!(preparation, Preparation::Running(ref request) if Arc::ptr_eq(request, &output.request))
            && self.phase == Phase::Writing
            && self.trial.visible
            && output.request.is_current(self.session.model());
        let retired = match output.result {
            Ok(prepared) if current => match self.session.accept_preview(prepared) {
                Ok(previous) => {
                    self.trial.answer.clear();
                    self.trial.choice_page = 0;
                    self.trial.effect_page = 0;
                    self.trial.status = "Preview uses the accepted source snapshot.".into();
                    previous.map(|preview| Retired::Preview(Box::new(preview)))
                }
                Err(prepared) => Some(Retired::Prepared(prepared)),
            },
            Ok(prepared) => {
                self.trial.status =
                    "Preview inputs changed or preparation was cancelled. Restart when ready."
                        .into();
                Some(Retired::Prepared(prepared))
            }
            Err(error) => {
                if current {
                    self.trial.status = error;
                }
                None
            }
        };
        // The receiver stays alive until this receipt or the window's queued output drops.
        let _ = output.receipt.send(retired);
    }
}

pub(super) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |state: &mut Writer, sender| state.trial.sender = Some(sender),
        Writer::accept_trial,
    )
}

async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Arc<PreviewRequest>>) {
    // The UI admits one request until its output is acknowledged, including after Cancel.
    // This bounds the toolkit's unbounded channel without creating a second task system.
    while let Some(request) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                request
                    .prepare(PreviewSetup::default())
                    .map_err(|error| error.to_string())
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
                match returned.recv().ok().flatten() {
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
