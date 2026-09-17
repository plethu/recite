//! Capture creation intent without writing source or replacing a catalogue.
use super::super::{
    create,
    messages::{MsgId, text as wording},
};
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;
use std::{path::PathBuf, sync::Arc};

pub(super) struct Pending {
    pub(super) preparation: create::Preparation,
    pub(super) path: PathBuf,
    pub(super) source: Arc<str>,
    pub(super) document: String,
}

pub(super) fn begin(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
    locale: &str,
) -> Result<Pending, String> {
    if writer.localisation.peek().dirty() {
        return Err(wording(MsgId::WriterOpenDrafts));
    }
    let locale = create::language(locale)?;
    writer.navigate(|_| Ok(()));
    if !writer.message.peek().is_empty() {
        return Err(writer.message.peek().clone());
    }
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
    let path = create::suggested_path(root, &locale.to_string());
    match std::fs::symlink_metadata(&path) {
        Ok(_) => return Err(wording(MsgId::WriterCatalogueExists)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
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
    Ok(Pending {
        preparation: create::Preparation::start(catalogue.to_pot_string(), locale)?,
        path,
        source: document.source_snapshot(),
        document: document.key().to_string(),
    })
}
