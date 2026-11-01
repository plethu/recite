//! Source and its resolved field publish together after all fallible preparation.
use super::{SourceValidation, View, Workbench};
use crate::{Document, DraftSnapshot};

pub(super) struct FieldState {
    view: View,
    script: Option<String>,
    draft: DraftSnapshot,
    loaded: DraftSnapshot,
    dirty: bool,
    validation: SourceValidation,
}
impl FieldState {
    pub fn take(model: Workbench) -> (Document, Self) {
        let Workbench {
            document,
            view,
            script_selection,
            draft,
            loaded,
            dirty,
            source_validation,
            ..
        } = model;
        (
            document,
            Self {
                view,
                script: script_selection,
                draft,
                loaded,
                dirty,
                validation: source_validation,
            },
        )
    }
    pub fn publish(&mut self, model: &mut Workbench) {
        std::mem::swap(&mut model.view, &mut self.view);
        std::mem::swap(&mut model.script_selection, &mut self.script);
        std::mem::swap(&mut model.draft, &mut self.draft);
        std::mem::swap(&mut model.loaded, &mut self.loaded);
        model.dirty = self.dirty;
        model.source_validation = self.validation;
        model.draft_revision = model.document.revision();
    }
}
