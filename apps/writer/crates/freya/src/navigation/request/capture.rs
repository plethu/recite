//! Capture immutable navigation inputs without copying editor sessions.
use super::{Request, Target};
use crate::{
    editing::{Presentation, Writer},
    navigation::{
        location::Screen,
        worker::{self, Destination},
    },
};
use recite_writer_model::{RecoveredDraft, View};
use std::{path::PathBuf, sync::Arc};
pub(super) struct Origin {
    source: recite_writer_model::CompletionSnapshot,
    draft: RecoveredDraft,
    presentation: Presentation,
    project: Option<(PathBuf, PathBuf)>,
    catalogue: Option<(PathBuf, Arc<()>)>,
}
impl Origin {
    pub(super) fn capture(writer: Writer) -> Result<Self, String> {
        let model = writer.buffers.model.peek();
        let model = model.as_ref().map_err(|e| e.to_string())?;
        Ok(Self {
            source: model.document().completion_snapshot(),
            draft: model.recovery(),
            presentation: Presentation::capture(writer),
            project: writer
                .files
                .peek()
                .as_ref()
                .map(|p| (p.root().to_owned(), p.current.clone())),
            catalogue: writer
                .localisation
                .peek()
                .catalogue
                .as_ref()
                .map(|c| (c.path.clone(), c.revision())),
        })
    }
    pub(super) fn is_current(&self, writer: Writer) -> bool {
        let model = writer.buffers.model.peek();
        model
            .as_ref()
            .is_ok_and(|m| self.source.is_current(m.document()) && self.draft == m.recovery())
            && self.presentation_is_current(writer)
    }
    pub(super) fn presentation_is_current(&self, writer: Writer) -> bool {
        self.presentation.is_current(writer)
            && self.project
                == writer
                    .files
                    .peek()
                    .as_ref()
                    .map(|p| (p.root().to_owned(), p.current.clone()))
            && match (
                &self.catalogue,
                writer.localisation.peek().catalogue.as_ref(),
            ) {
                (None, None) => true,
                (Some((path, revision)), Some(c)) => {
                    path == &c.path && Arc::ptr_eq(revision, &c.revision())
                }
                _ => false,
            }
    }
}
impl Request {
    pub(super) fn capture(&self, writer: Writer) -> Result<worker::Prepare, String> {
        let files = writer.files.peek();
        let model = writer.buffers.model.peek();
        let model = model.as_ref().map_err(|e| e.to_string())?;
        let location = match &self.target {
            Target::Location(l) => Some(l.clone()),
            _ => None,
        };
        let destination = match &self.target {
            Target::Document { path, view, .. } => Destination::Switch(Box::new(
                files
                    .as_ref()
                    .ok_or("Open a project first.")?
                    .switch_request(
                        model,
                        path,
                        (*view == recite_config::WriterView::Source).then_some(View::Source),
                    )
                    .map_err(|e| e.to_string())?,
            )),
            Target::Location(l)
                if !l.document.is_empty() && l.document != model.document().key().as_str() =>
            {
                if let Some(files) = files.as_ref() {
                    let path = files
                        .path_for_document(&l.document)
                        .ok_or("The linked scene is not in this project.")?;
                    Destination::Switch(Box::new(
                        files
                            .switch_request(
                                model,
                                &path,
                                (l.source || l.screen == Screen::Rules).then_some(View::Source),
                            )
                            .map_err(|e| e.to_string())?,
                    ))
                } else {
                    if !recite_writer_model::WRITER_EXAMPLES
                        .iter()
                        .any(|e| e.name == l.document)
                    {
                        return Err("The linked scene is not open.".into());
                    }
                    Destination::Example {
                        name: l.document.clone(),
                        retained: writer
                            .examples
                            .peek()
                            .get(&l.document)
                            .map(|m| m.navigation_request()),
                    }
                }
            }
            _ => Destination::Current(model.navigation_request()),
        };
        let catalogue = writer
            .localisation
            .peek()
            .catalogue
            .as_ref()
            .map(|c| c.path.clone());
        let catalogue_changes = location.as_ref().is_some_and(|l| {
            l.catalogue.as_ref().map(|c| {
                files
                    .as_ref()
                    .map_or_else(|| PathBuf::from(c), |f| f.root().join(c))
            }) != catalogue
        });
        if catalogue_changes && writer.localisation.peek().dirty() {
            return Err(crate::localisation::close_drafts_message());
        }
        Ok(worker::Prepare {
            destination,
            location: location.clone(),
            project: files.as_ref().map(|f| f.root().to_owned()),
            catalogue,
            reuse_rules: location.as_ref().is_some_and(|l| {
                l.screen == Screen::Rules
                    && (l.document.is_empty() || l.document == model.document().key().as_str())
                    && writer.buffers.rules.peek().as_ref().is_some_and(|r| {
                        Some(&r.passage) == l.passage.as_ref() && r.belongs_to(model.document())
                    })
            }),
            initial_view: match self.target {
                Target::Document {
                    view,
                    initial: true,
                    ..
                } => Some(view),
                _ => None,
            },
        })
    }
}
