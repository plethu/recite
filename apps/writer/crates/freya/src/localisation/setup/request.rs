//! Capture and revalidate catalogue creation intent at the persistence boundary.
use super::super::{
    catalogue::Catalogue,
    create,
    messages::{MsgId, text as wording},
};
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;
use recite_core::po::PoDocument;
use std::{path::PathBuf, sync::Arc};

pub(super) struct Pending {
    pub(super) preparation: create::Preparation,
    path: PathBuf,
    source: Arc<str>,
    document: String,
    template: String,
}

impl Pending {
    pub(super) fn persist(
        self,
        writer: Writer,
        files: State<Option<ProjectFiles>>,
        result: Result<PoDocument, String>,
    ) -> Result<Catalogue, String> {
        let unchanged = writer.buffers.model.peek().as_ref().is_ok_and(|model| {
            model.document().key().as_str() == self.document
                && model.document().source() == self.source.as_ref()
                && !model.has_draft()
        });
        if !unchanged || writer.localisation.peek().dirty() {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        let document = result?;
        if super::super::extraction::template(writer, files)? != self.template {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        create::persist(&document, &self.path)
    }
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
