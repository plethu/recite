//! One pending destination owns preparation, freshness and completion.
mod capture;
mod publication;
use super::{location::Location, worker};
use crate::{
    editing::Writer,
    editor_work::worker::{Output, Work, Worker},
};
use capture::Origin;
use recite_writer_model::{EditOperation, View};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) enum Target {
    Document {
        path: PathBuf,
        view: recite_config::WriterView,
        initial: bool,
    },
    Location(Location),
}
pub(crate) struct Request {
    pub target: Target,
    pub cancelled: Option<Arc<AtomicBool>>,
}
pub(crate) struct Navigation {
    request: Request,
    origin: Origin,
    prepared_origin: Option<Origin>,
    prepared: Option<Box<worker::Prepared>>,
}
impl Request {
    pub(crate) fn begin(self, writer: Writer) -> Result<Navigation, String> {
        writer.buffers.harvest();
        let origin = Origin::capture(writer)?;
        self.require_current(&origin, writer)?;
        Ok(Navigation {
            request: self,
            origin,
            prepared_origin: None,
            prepared: None,
        })
    }
    fn require_not_cancelled(&self) -> Result<(), String> {
        if self
            .cancelled
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Acquire))
        {
            return Err("Writer activation was cancelled".into());
        }
        Ok(())
    }
    fn require_presentation(&self, origin: &Origin, writer: Writer) -> Result<(), String> {
        self.require_not_cancelled()?;
        if !origin.presentation_is_current(writer) {
            return Err(
                "The view changed while navigation was preparing. Retry navigation.".into(),
            );
        }
        Ok(())
    }
    fn require_current(&self, origin: &Origin, writer: Writer) -> Result<(), String> {
        if self
            .cancelled
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Acquire))
        {
            return Err("Writer activation was cancelled".into());
        }
        if !origin.is_current(writer) {
            return Err("The current draft or view changed while navigation was preparing. Your edits remain open; retry navigation.".into());
        }
        Ok(())
    }
}

pub(crate) async fn execute(
    writer: Writer,
    mut navigation: Box<Navigation>,
    worker: &Worker,
) -> Result<(), String> {
    let result = execute_prepared(writer, &mut navigation, worker).await;
    // Origins and the prepared/displaced destination retire together on every outcome.
    worker
        .execute(Work::Navigation(Box::new(worker::Work::RetireRequest(
            navigation,
        ))))
        .await
        .map_err(|e| e.to_string())?;
    result
}

async fn execute_prepared(
    writer: Writer,
    navigation: &mut Navigation,
    worker: &Worker,
) -> Result<(), String> {
    let Navigation {
        request,
        origin: intent,
        prepared_origin,
        prepared,
    } = navigation;
    writer.buffers.harvest();
    request.require_current(intent, writer)?;
    if matches!(request.target, Target::Location(_))
        && writer
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| matches!(m.view(), View::Passage(_)) && m.has_draft())
    {
        if let Some(edit) = crate::editor_work::document::capture(writer, EditOperation::Apply)? {
            let mut prepared = crate::editor_work::document::prepare(edit, worker)
                .await
                .map_err(|e| e.to_string())?;
            let result = request.require_presentation(intent, writer).and_then(|()| {
                crate::editor_work::document::publish(writer.buffers, writer.dark, &mut prepared)
                    .map_err(|e| e.to_string())
            });
            worker
                .execute(Work::RetireEdit(prepared))
                .await
                .map_err(|e| e.to_string())?;
            result?;
        }
        if writer
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| m.has_draft())
        {
            return Err(recite_writer_model::WorkbenchError::DraftPending.to_string());
        }
    }
    request.require_presentation(intent, writer)?;
    let origin = prepared_origin.insert(Origin::capture(writer)?);
    request.require_current(origin, writer)?;
    let preparation = request.capture(writer)?;
    let Output::Navigation(worker::Output::Prepared(result)) = worker
        .execute(Work::Navigation(Box::new(worker::Work::Prepare(Box::new(
            preparation,
        )))))
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Invalid navigation preparation receipt.".into());
    };
    let prepared = prepared.insert(result?);
    publication::publish(writer, request, origin, prepared, worker).await
}

#[cfg(test)]
mod tests;
