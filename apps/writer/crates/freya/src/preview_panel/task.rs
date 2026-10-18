//! Widget capture and model publication are the only UI work in trial startup.
use super::{
    Snapshot,
    prepare::Inputs,
    request::Request,
    worker::{Output, Retired, Worker},
};
use crate::{
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
use std::sync::Arc;

pub(super) struct Work {
    worker: Worker,
    pending: Option<Arc<Request>>,
}
impl Work {
    fn finish(&mut self, request: &Arc<Request>) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, request))
        {
            self.pending = None;
            true
        } else {
            false
        }
    }
}
pub(crate) fn use_preview_work(mut writer: Writer) {
    let receiver = use_hook(move || {
        let worker = Worker::new();
        let receiver = worker.results();
        writer.trial.work.set(Some(Work {
            worker,
            pending: None,
        }));
        receiver
    });
    use_future(move || {
        let receiver = receiver.clone();
        async move {
            while let Ok(output) = receiver.recv().await {
                accept(writer, output);
            }
        }
    });
    use_side_effect(move || {
        if *writer.pane.read() != Pane::Preview {
            cancel(writer);
        }
    });
    use_drop(move || writer.trial.work.set(None));
}
pub(crate) fn start(writer: Writer, configured: bool) -> Result<(), String> {
    writer.work.require_idle()?;
    writer.after_prose(move |writer| start_applied(writer, configured));
    Ok(())
}
fn start_applied(mut writer: Writer, configured: bool) -> Result<(), String> {
    writer.try_navigate(|_| Ok(()))?;
    let inputs = Inputs::capture(writer, configured);
    let model = writer.buffers.model.peek();
    let model = model.as_ref().map_err(|e| e.to_string())?;
    let snapshot = Snapshot::capture(
        format!(
            "{} · {} · {} {}",
            if inputs.locale.is_empty() {
                text(MsgId::WriterSourceOnly)
            } else {
                inputs.locale.clone()
            },
            text(if inputs.include_drafts {
                MsgId::WriterTrialDrafts
            } else {
                MsgId::WriterTrialSaved
            }),
            text(MsgId::WriterSourceRevision),
            model.document().revision()
        ),
        !inputs.locale.is_empty(),
        writer.localisation.peek().catalogue.as_ref(),
    );
    let request = Arc::new(Request {
        source: model.preview_request().map_err(|e| e.to_string())?,
        view: model.view().clone(),
        inputs,
        snapshot,
    });
    let mut work = writer.trial.work.write();
    let work = work.as_mut().ok_or("Preview worker unavailable")?;
    work.worker
        .submit(request.clone())
        .map_err(|e| e.to_string())?;
    work.pending = Some(request);
    writer.pane.set(Pane::Preview);
    Ok(())
}
pub(crate) fn cancel(mut writer: Writer) {
    if let Some(work) = writer.trial.work.write().as_mut() {
        work.worker.cancel();
        work.pending = None;
    }
}
pub(super) fn pending(writer: Writer) -> bool {
    writer
        .trial
        .work
        .read()
        .as_ref()
        .is_some_and(|w| w.pending.is_some())
}
fn accept(mut writer: Writer, output: Output) {
    let Output {
        request,
        result,
        receipt,
    } = output;
    let mut work = writer.trial.work.write();
    let current = work.as_mut().is_some_and(|work| work.finish(&request));
    drop(work);
    if !current || *writer.pane.peek() != Pane::Preview {
        let _ = receipt.try_send((request, result.ok().map(Retired::Prepared)));
        return;
    }
    writer.buffers.harvest();
    let mut model = writer.buffers.model.write();
    let current_inputs = model.as_ref().is_ok_and(|m| {
        request.is_current(
            m,
            *writer.pane.peek(),
            writer.localisation.peek().catalogue.as_ref(),
        )
    });
    if !current_inputs {
        writer.message.info(text(MsgId::WriterPreviewInputsChanged));
    }
    let current = current_inputs;
    let retired = match result {
        Ok(prepared) if current => match model.as_mut() {
            Ok(model) => match model.accept_preview(prepared) {
                Ok(previous) => {
                    writer.trial.snapshot.set(Some(request.snapshot.clone()));
                    previous.map(|preview| Retired::Preview(Box::new(preview)))
                }
                Err(stale) => Some(Retired::Prepared(stale)),
            },
            Err(_) => Some(Retired::Prepared(prepared)),
        },
        Ok(stale) => Some(Retired::Prepared(stale)),
        Err(error) => {
            if current {
                writer.message.error(error);
            }
            None
        }
    };
    // Exactly one receipt follows each output. The worker frees its previous
    // session before processing another queued request.
    let _ = receipt.try_send((request, retired));
}

#[cfg(test)]
mod tests;
