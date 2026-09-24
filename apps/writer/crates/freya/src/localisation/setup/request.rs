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
    pub(super) template: String,
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
    let template = super::super::extraction::template(writer, files)?;
    let session = writer.buffers.model.peek();
    let session = session.as_ref().map_err(|e| e.to_string())?;
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
    Ok(Pending {
        preparation: create::Preparation::start(template.clone(), locale)?,
        template,
        path,
        source: document.source_snapshot(),
        document: document.key().to_string(),
    })
}
