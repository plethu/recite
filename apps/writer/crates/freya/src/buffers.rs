//! Open text buffers own harvesting, save and scene-switch protection.
use crate::{
    editing::{Session, editor_data},
    project::ProjectFiles,
};
use freya::{code_editor::CodeEditorData, prelude::*};
use recite_writer_model::View;
#[derive(Clone, Copy)]
pub(crate) struct Buffers {
    pub model: Session,
    pub editor: State<CodeEditorData>,
    pub prose: State<String>,
}

impl Buffers {
    pub(super) fn install(mut self, next: recite_writer_model::Workbench, dark: bool) {
        self.editor.set(editor_data(
            next.draft(),
            next.view() == &View::Source,
            dark,
        ));
        self.prose.set(next.draft().to_owned());
        self.model.set(Ok(next));
    }

    pub(super) fn harvest(mut self) {
        if let Ok(model) = self.model.write().as_mut() {
            model.set_draft(if model.view() == &View::Source {
                self.editor.peek().rope.to_string()
            } else {
                self.prose.peek().clone()
            });
        }
    }

    pub(super) fn save(self, mut files: State<Option<ProjectFiles>>) -> Result<(), String> {
        self.harvest();
        let mut model = self.model;
        let mut state = model.write();
        let workbench = state.as_mut().map_err(|error| error.to_string())?;
        let mut opened = files.write();
        let project = opened.as_mut().ok_or("Open a project before saving.")?;
        workbench.apply().map_err(|error| error.to_string())?;
        project
            .save(workbench.document().source())
            .map_err(|error| error.to_string())?;
        project
            .checkpoint(workbench)
            .map_err(|error| error.to_string())
    }

    pub(super) fn can_leave(self, files: Option<&ProjectFiles>) -> bool {
        self.harvest();
        self.model
            .peek()
            .as_ref()
            .is_ok_and(|m| !m.has_draft() && files.is_none_or(|f| !f.dirty(m.document().source())))
    }
}
