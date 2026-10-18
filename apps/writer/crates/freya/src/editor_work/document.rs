//! Checked edit receipts preserve live Source widgets and restore prepared history views.
use super::worker::{Output, Work, Worker};
use crate::{
    editing::{Pane, Writer},
    project::FileError,
};
use freya::prelude::*;
use recite_writer_model::{EditOperation, EditRequest, PreparedEdit, ReplyRulesRequest, View};

pub(crate) fn capture(
    writer: Writer,
    operation: EditOperation,
) -> Result<Option<EditRequest>, String> {
    let model = writer.buffers.model.peek();
    let model = model.as_ref().map_err(|e| e.to_string())?;
    let request = model.edit_request(operation).map_err(|e| e.to_string())?;
    Ok(request.map(|request| {
        if operation == EditOperation::Apply
            && model.view() == &View::Source
            && let Some(rules) = writer.buffers.rules.peek().as_ref()
        {
            request.with_rules(rules.clone())
        } else {
            request
        }
    }))
}
pub(crate) async fn accept(
    writer: Writer,
    request: EditRequest,
    worker: &Worker,
) -> Result<(), FileError> {
    let mut prepared = prepare(request, worker).await?;
    let result = publish(writer.buffers, writer.dark, &mut prepared);
    worker.execute(Work::RetireEdit(prepared)).await?;
    result
}
pub(crate) async fn prepare(
    request: EditRequest,
    worker: &Worker,
) -> Result<Box<PreparedEdit>, FileError> {
    let Output::Edit(result) = worker.execute(Work::Edit(request)).await? else {
        return Err(FileError::Selection);
    };
    result.map_err(FileError::from)
}
pub(crate) fn publish(
    mut buffers: crate::buffers::Buffers,
    dark: bool,
    prepared: &mut PreparedEdit,
) -> Result<(), FileError> {
    buffers.harvest();
    let was_source = buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|model| model.view() == &View::Source);
    let result = buffers
        .model
        .write()
        .as_mut()
        .map_err(|_| FileError::Selection)
        .and_then(|model| model.accept_edit(prepared).map_err(FileError::from));
    // Source Apply keeps its Rope, caret and widget undo history. History also
    // restores the selected prose field or switches the widget back to Source.
    if result.is_ok() {
        let model = buffers.model.peek();
        if let Ok(model) = model.as_ref() {
            let source = model.view() == &View::Source;
            if !source
                || source != was_source
                || !model
                    .draft_snapshot()
                    .matches_rope(&buffers.editor.peek().rope)
            {
                buffers
                    .editor
                    .set(crate::editing::editor_data(model.draft(), source, dark));
                if source {
                    buffers.restore();
                }
            }
            buffers.prose.set(crate::buffers::prose_draft(model).into());
        }
    }
    result
}
pub(super) async fn rules(
    mut writer: Writer,
    request: ReplyRulesRequest,
    worker: &Worker,
) -> Result<(), String> {
    let Output::Rules(request, result) = worker
        .execute(Work::Rules(request))
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Invalid Rules worker receipt.".into());
    };
    writer.buffers.harvest();
    let current = *writer.pane.peek() == Pane::Rules
        && writer.buffers.model.peek().as_ref().is_ok_and(|m| {
            m.view() == &View::Source && !m.has_draft() && request.is_current(m.document())
        });
    if current && let Ok(rules) = &result {
        writer.buffers.rules.set(Some(rules.as_ref().clone()));
    }
    let error = if current {
        result.as_ref().err().cloned()
    } else {
        None
    };
    worker
        .execute(Work::RetireRules(Box::new((request, result.ok()))))
        .await
        .map_err(|e| e.to_string())?;
    error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests;
