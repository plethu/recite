//! Resolve the current field, then use shared catalogue extraction policy.
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;
use recite_compiler::authoring::CancellationToken;
use recite_writer_session::localisation::extraction;

pub(super) fn template(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
) -> Result<String, String> {
    writer.try_navigate(|_| Ok(()))?;
    let model = writer.buffers.model.peek();
    let model = model.as_ref().map_err(|e| e.to_string())?;
    let files = files.peek();
    extraction::template(model, files.as_ref(), &CancellationToken::new())
}
