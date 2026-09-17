//! Capture the current project and source overlay for catalogue operations.
use super::messages::{MsgId, text as wording};
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;

pub(super) fn template(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
) -> Result<String, String> {
    writer.try_navigate(|_| Ok(()))?;
    let session = writer.buffers.model.peek();
    let session = session.as_ref().map_err(|e| e.to_string())?;
    if session.has_draft() {
        return Err(wording(MsgId::WriterCreateSourceDraft));
    }
    let document = session.document();
    let project = files.peek();
    let root = project
        .as_ref()
        .map_or(std::path::Path::new("."), |p| p.root());
    // Refresh other saved scenes, retaining the current unsaved source overlay.
    let report = if project.is_some() {
        let discovered = recite_config::discover_project(root).map_err(|e| e.to_string())?;
        if !discovered.is_complete() {
            return Err(wording(MsgId::WriterCreateIncomplete));
        }
        let context = crate::project_context::load(&discovered).map_err(|e| e.to_string())?;
        let snapshot = recite_writer_model::Document::in_project(
            document.key().clone(),
            document.source(),
            context,
        )
        .map_err(|e| e.to_string())?;
        snapshot.extract_catalogue()
    } else {
        document.extract_catalogue()
    };
    let catalogue = report
        .catalog
        .ok_or_else(|| crate::project_context::diagnostic_messages(&report.diagnostics))?;
    if catalogue.entries.is_empty() {
        return Err(wording(MsgId::WriterCreateEmpty));
    }
    Ok(catalogue.to_pot_string())
}
