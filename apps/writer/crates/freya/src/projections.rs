//! Demand local reading projections without parsing accepted source on the UI.
mod worker;
use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_writer_model::{ProjectionRequest, View};
use worker::{Output, Worker};

struct ViewIntent {
    origin: View,
    pane: Pane,
    view: recite_config::WriterView,
    split: Option<bool>,
}
impl ViewIntent {
    fn is_current(&self, model: &recite_writer_model::Workbench, pane: Pane) -> bool {
        !model.has_draft() && model.view() == &self.origin && pane == self.pane
    }
}
pub(crate) struct Work {
    worker: Worker,
    pending: Option<ProjectionRequest>,
    failure: Option<(ProjectionRequest, String)>,
    intent: Option<ViewIntent>,
}

pub(crate) fn use_work(mut writer: Writer) {
    let receiver = use_hook(move || {
        let worker = Worker::new();
        let receiver = worker.results();
        writer.projections.set(Some(Work {
            worker,
            pending: None,
            failure: None,
            intent: None,
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
        if needed(writer) {
            request(writer);
        }
    });
    use_drop(move || writer.projections.set(None));
}

pub(crate) fn needed(writer: Writer) -> bool {
    if !matches!(*writer.pane.read(), Pane::Map | Pane::Script) {
        return false;
    }
    let localisation = writer.localisation.read();
    if localisation.active && localisation.view == crate::localisation::CatalogueView::Updates {
        return false;
    }
    writer.buffers.model.read().as_ref().is_ok_and(|m| {
        m.view() != &View::Source
            || localisation.active
            || (crate::scene::has_no_beats(m)
                && *writer.layout.view.read() != recite_config::WriterView::Source)
    })
}

fn request(mut writer: Writer) {
    let model = writer.buffers.model.read();
    let Ok(model) = model.as_ref() else { return };
    let document = model.document();
    if document.cached_script().is_some() {
        return;
    }
    let mut state = writer.projections.write();
    let Some(work) = state.as_mut() else { return };
    if work
        .pending
        .as_ref()
        .is_some_and(|r| r.is_current(document))
        || work
            .failure
            .as_ref()
            .is_some_and(|(r, _)| r.is_current(document))
    {
        return;
    }
    let request = document.projection_request();
    match work.worker.submit(request.clone()) {
        Ok(()) => {
            work.pending = Some(request);
            work.failure = None;
        }
        Err(error) => {
            writer.message.error(error.to_string());
            work.failure = Some((request, error.to_string()));
        }
    }
}

pub(crate) fn ready(writer: Writer) -> bool {
    writer
        .buffers
        .model
        .read()
        .as_ref()
        .is_ok_and(|m| m.document().cached_script().is_some())
}
pub(crate) fn placeholder(writer: Writer) -> Element {
    let model = writer.buffers.model.read();
    let work = writer.projections.read();
    let error = work
        .as_ref()
        .and_then(|w| w.failure.as_ref())
        .filter(|(r, _)| model.as_ref().is_ok_and(|m| r.is_current(m.document())));
    let caption = error.map_or_else(
        || crate::messages::text(crate::messages::MsgId::WriterPreparingScript),
        |(_, e)| e.clone(),
    );
    rect()
        .width(Size::fill())
        .padding(crate::design::tokens::SPACE_MD)
        .a11y_role(AccessibilityRole::Status)
        .a11y_alt(caption.clone())
        .child(label().text(caption))
        .into_element()
}
/// Defer only the explicit view command; routes keep their own success policy.
pub(crate) fn defer_view(
    mut writer: Writer,
    view: recite_config::WriterView,
    split: Option<bool>,
) -> bool {
    cancel_view(writer);
    if view == recite_config::WriterView::Source {
        return false;
    }
    if writer
        .try_navigate(|m| {
            if m.has_draft() {
                Err(recite_writer_model::WorkbenchError::DraftPending)
            } else {
                Ok(())
            }
        })
        .is_err()
    {
        return true;
    }
    if ready(writer) {
        return false;
    }
    request(writer);
    let error = {
        let model = writer.buffers.model.peek();
        let state = writer.projections.peek();
        state
            .as_ref()
            .and_then(|w| w.failure.as_ref())
            .filter(|(request, _)| {
                model
                    .as_ref()
                    .is_ok_and(|m| request.is_current(m.document()))
            })
            .map(|(_, error)| error.clone())
    };
    if let Some(error) = error {
        writer.message.error(error);
        return true;
    }
    let model = writer.buffers.model.peek();
    if let Ok(model) = model.as_ref()
        && let Some(work) = writer.projections.write().as_mut()
        && work
            .pending
            .as_ref()
            .is_some_and(|r| r.is_current(model.document()))
    {
        work.intent = Some(ViewIntent {
            origin: model.view().clone(),
            pane: *writer.pane.peek(),
            view,
            split,
        });
    }
    true
}
pub(crate) fn cancel_view(mut writer: Writer) {
    if let Some(work) = writer.projections.write().as_mut() {
        work.intent = None;
    }
}
pub(crate) fn pending_view(writer: Writer) -> bool {
    writer
        .projections
        .read()
        .as_ref()
        .is_some_and(|w| w.intent.is_some())
}
fn accept(mut writer: Writer, output: Output) {
    let Output {
        request,
        result,
        receipt,
    } = output;
    writer.buffers.harvest();
    let mut state = writer.projections.write();
    let model = writer.buffers.model.write();
    let current = model
        .as_ref()
        .is_ok_and(|m| request.is_current(m.document()));
    let mut intent = None;
    if let Some(work) = state.as_mut() {
        // An old receipt must not clear a newer request.
        let latest = work
            .pending
            .as_ref()
            .is_some_and(|r| r.same_source(&request));
        if latest {
            work.pending = None;
            intent = work.intent.take().filter(|i| {
                current
                    && model
                        .as_ref()
                        .is_ok_and(|m| i.is_current(m, *writer.pane.peek()))
            });
        }
        if current {
            match &result {
                Ok(prepared) => {
                    if let Ok(model) = model.as_ref() {
                        model.document().accept_projection(prepared);
                    }
                }
                Err(error) => {
                    work.failure = Some((request.clone(), error.clone()));
                }
            }
        }
    }
    let succeeded = current && result.is_ok();
    let error = (current && intent.is_some())
        .then(|| result.as_ref().err().cloned())
        .flatten();
    drop(model);
    drop(state);
    // Accepted arrays are shared with the cache; stale arrays are freed by the worker.
    let _ = receipt.try_send((request, result.ok()));
    if let Some(error) = error {
        writer.message.error(error);
    }
    if succeeded && let Some(intent) = intent {
        writer.finish_view(intent.view, intent.split);
    }
}

#[cfg(test)]
mod tests;
