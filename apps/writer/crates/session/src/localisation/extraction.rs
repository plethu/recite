//! Fresh saved scenes plus accepted unsaved edits form one catalogue source.
use super::messages::{MsgId, text as wording};
use crate::project::ProjectFiles;
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{Document, Workbench};

/// Performs discovery and analysis; callers run this outside their event loop.
pub fn template(
    model: &Workbench,
    files: Option<&ProjectFiles>,
    control: &dyn WorkControl,
) -> Result<String, String> {
    control.checkpoint().map_err(|error| error.to_string())?;
    if model.has_draft() {
        return Err(wording(MsgId::WriterCreateSourceDraft));
    }
    let document = model.document();
    let report = if let Some(files) = files {
        let discovered =
            recite_config::discover_project(files.root()).map_err(|e| e.to_string())?;
        if !discovered.is_complete() {
            return Err(wording(MsgId::WriterCreateIncomplete));
        }
        if discovered.manifest().project_root() != files.root() {
            return Err(crate::FileError::ProjectRootChanged.to_string());
        }
        for path in files.session_paths() {
            if !discovered
                .documents()
                .iter()
                .any(|document| document.path() == path)
            {
                return Err(crate::FileError::SessionExcluded(path.clone()).to_string());
            }
        }
        control.checkpoint().map_err(|error| error.to_string())?;
        let context = files
            .catalogue_context(crate::load_context(&discovered).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Document::in_project(document.key().clone(), document.source(), context)
            .map_err(|e| e.to_string())?
            .extract_catalogue()
    } else {
        document.extract_catalogue()
    };
    control.checkpoint().map_err(|error| error.to_string())?;
    let catalogue = report
        .catalog
        .ok_or_else(|| crate::diagnostic_messages(&report.diagnostics))?;
    if catalogue.entries.is_empty() {
        return Err(wording(MsgId::WriterCreateEmpty));
    }
    Ok(catalogue.to_pot_string())
}
