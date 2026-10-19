//! Worker preparation restores selection before checked, allocation-light publication.
use super::{View, Workbench, WorkbenchError};
use crate::{
    DraftSnapshot, EditError, EditOperation, PreparedProjection,
    document::{PreparedSourceEdit, SourceEditRequest},
};
use recite_compiler::authoring::{AuthoringError, WorkControl};

pub struct EditRequest {
    source: SourceEditRequest,
    selection: Option<Selection>,
    operation: EditOperation,
}
pub struct PreparedEdit {
    source: PreparedSourceEdit,
    selection: Option<PreparedSelection>,
    applying: bool,
}
struct Selection {
    view: View,
    script: Option<String>,
}
struct PreparedSelection {
    origin: Selection,
    resolved: Selection,
    draft: DraftSnapshot,
    projection: Option<PreparedProjection>,
}
impl Workbench {
    /// Capture Source/prose Apply or local history without materializing text or projections.
    pub fn edit_request(
        &self,
        operation: EditOperation,
    ) -> Result<Option<EditRequest>, WorkbenchError> {
        if operation != EditOperation::Apply {
            self.require_applied()?;
        }
        self.document.check_revision(self.draft_revision)?;
        let source = match (&operation, &self.view) {
            (EditOperation::Apply, View::Passage(id)) => self.has_draft().then(|| {
                self.document
                    .prose_edit_request(self.draft.clone(), id.clone())
            }),
            (EditOperation::Apply, View::Block(_)) => None,
            _ => self
                .document
                .source_edit_request(self.draft.clone(), operation),
        };
        Ok(source.map(|source| EditRequest {
            source,
            selection: (self.view != View::Source).then(|| Selection {
                view: self.view.clone(),
                script: self.script_selection.clone(),
            }),
            operation,
        }))
    }

    /// Keep the receipt until its displaced source, projections and history can retire off UI.
    pub fn accept_edit(&mut self, prepared: &mut PreparedEdit) -> Result<(), WorkbenchError> {
        let submitted_is_current = self.draft == prepared.source.request.draft;
        if let Some(selection) = &prepared.selection {
            if self.view != selection.origin.view
                || self.script_selection != selection.origin.script
            {
                return Err(EditError::Stale.into());
            }
            if !prepared.applying {
                self.require_applied()?;
            }
            if !prepared.applying && !submitted_is_current {
                return Err(EditError::Stale.into());
            }
        } else if self.view != View::Source {
            return Err(EditError::Stale.into());
        }
        // Check freshness before advancing history or replacing the selected field.
        self.document.accept_source_edit(&mut prepared.source)?;
        if let Some(selection) = &mut prepared.selection {
            if let Some(projection) = &selection.projection {
                self.document.accept_projection(projection);
            }
            std::mem::swap(&mut self.view, &mut selection.resolved.view);
            std::mem::swap(&mut self.script_selection, &mut selection.resolved.script);
            self.loaded = selection.draft.clone();
            if submitted_is_current {
                self.draft = self.loaded.clone();
            }
        } else {
            self.loaded = prepared.source.source.clone().into();
            if submitted_is_current {
                self.draft = self.loaded.clone();
            }
        }
        self.dirty = self.draft != self.loaded;
        self.draft_revision = self.document.revision();
        Ok(())
    }
}
impl EditRequest {
    pub fn with_rules(mut self, rules: crate::ReplyRules) -> Self {
        self.source = self.source.with_rules(rules);
        self
    }
    pub fn prepare(self, control: &dyn WorkControl) -> Result<PreparedEdit, WorkbenchError> {
        let applying = self.operation == EditOperation::Apply;
        let source = self.source.prepare(control)?;
        let selection = self
            .selection
            .map(|origin| {
                let document = source.projection_document();
                let projection = (!applying)
                    .then(|| document.projection_request().prepare().ok())
                    .flatten();
                if let Some(projection) = &projection {
                    document.accept_projection(projection);
                }
                let mut model = Workbench {
                    draft_revision: document.revision(),
                    document,
                    view: origin.view.clone(),
                    draft: String::new().into(),
                    loaded: String::new().into(),
                    dirty: false,
                    preview: None,
                    page: None,
                    script_selection: origin.script.clone(),
                };
                // History may restore malformed source. Preserve the existing fallback policy.
                model.refresh()?;
                Ok::<_, WorkbenchError>(PreparedSelection {
                    origin,
                    resolved: Selection {
                        view: model.view,
                        script: model.script_selection,
                    },
                    draft: model.draft,
                    projection,
                })
            })
            .transpose()?;
        control
            .checkpoint()
            .map_err(|error| EditError::from(AuthoringError::from(error)))?;
        Ok(PreparedEdit {
            source,
            selection,
            applying,
        })
    }
}
