//! Open text buffers own harvesting, save and scene-switch protection.
mod bookmarks;
use crate::{
    editing::{Session, editor_data},
    project::ProjectFiles,
};
pub(crate) use bookmarks::Bookmarks;
use freya::{code_editor::CodeEditorData, prelude::*};
use recite_writer_model::View;
#[derive(Clone, Copy)]
pub(crate) struct Buffers {
    pub bookmarks: Bookmarks,
    pub rules: State<Option<recite_writer_model::ReplyRules>>,
    pub model: Session,
    pub editor: State<CodeEditorData>,
    pub prose: State<String>,
}

impl Buffers {
    pub(crate) fn sync(mut self, dark: bool) {
        let state = self.model.peek();
        if let Ok(model) = state.as_ref() {
            self.editor.set(editor_data(
                model.draft(),
                model.view() == &View::Source,
                dark,
            ));
            self.prose.set(model.draft().into());
        }
    }

    pub(super) fn install(mut self, next: recite_writer_model::Workbench, dark: bool) {
        self.editor.set(editor_data(
            next.draft(),
            next.view() == &View::Source,
            dark,
        ));
        self.prose.set(next.draft().to_owned());
        self.model.set(Ok(next));
    }

    pub(super) fn remember(self) {
        if let Ok(model) = self.model.peek().as_ref()
            && model.view() == &View::Source
        {
            self.bookmarks
                .remember(model.document().key().as_str(), &self.editor.peek());
        }
    }
    pub(super) fn restore(mut self) {
        if let Ok(model) = self.model.peek().as_ref()
            && model.view() == &View::Source
        {
            self.bookmarks
                .restore(model.document().key().as_str(), &mut self.editor.write());
        }
    }
    pub(super) fn harvest(mut self) {
        self.remember();
        if let Ok(model) = self.model.write().as_mut() {
            model.set_draft(if model.view() == &View::Source {
                self.editor.peek().rope.to_string()
            } else {
                self.prose.peek().clone()
            });
        }
    }

    pub(super) fn save(mut self, mut files: State<Option<ProjectFiles>>) -> Result<(), String> {
        self.harvest();
        let mut model = self.model;
        let mut state = model.write();
        let workbench = state.as_mut().map_err(|error| error.to_string())?;
        let mut opened = files.write();
        let project = opened.as_mut().ok_or("Open a project before saving.")?;
        if let Some(rules) = self.rules.peek().as_ref()
            && rules.belongs_to(workbench.document())
            && rules.changed()
            && rules
                .source()
                .is_ok_and(|source| source == workbench.draft())
        {
            rules
                .validate(workbench.document())
                .map_err(|error| error.to_string())?;
        }
        workbench.apply().map_err(|error| error.to_string())?;
        let reply = self
            .rules
            .peek()
            .as_ref()
            .map(|rules| rules.passage.clone());
        if let Some(reply) = reply {
            self.rules
                .set(workbench.document().reply_rules(&reply).ok());
        }
        let grouped = project.project_edit_pending(workbench);
        project
            .save(workbench.document().source())
            .map_err(|error| error.to_string())?;
        project
            .checkpoint(workbench)
            .map_err(|error| error.to_string())?;
        if grouped {
            project.save_retained().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(super) fn switch(
        self,
        mut files: State<Option<ProjectFiles>>,
        path: &std::path::Path,
        dark: bool,
        select: impl FnOnce(
            &mut recite_writer_model::Workbench,
        ) -> Result<(), recite_writer_model::WorkbenchError>,
    ) -> Result<(), String> {
        self.harvest();
        let mut state = self.model;
        let mut model = state.write();
        let model = model.as_mut().map_err(|e| e.to_string())?;
        files
            .write()
            .as_mut()
            .ok_or("Open a project first.")?
            .switch(model, path, select)
            .map_err(|e| e.to_string())?;
        let mut editor = self.editor;
        editor.set(editor_data(
            model.draft(),
            model.view() == &View::Source,
            dark,
        ));
        if model.view() == &View::Source {
            self.bookmarks
                .restore(model.document().key().as_str(), &mut editor.write());
        }
        let mut prose = self.prose;
        prose.set(model.draft().into());
        Ok(())
    }

    pub(super) fn save_all(mut self, mut files: State<Option<ProjectFiles>>) -> Result<(), String> {
        self.save(files)?;
        let mut files = files.write();
        let project = files.as_mut().ok_or("Open a project first.")?;
        project.save_retained().map_err(|e| e.to_string())?;
        if let Some(session) = &mut project.declarations
            && session.dirty()
        {
            session.save_and_generate().map_err(|e| e.to_string())?;
            if let Ok(model) = self.model.write().as_mut() {
                project.refresh(model).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub(super) fn can_leave(self, files: Option<&ProjectFiles>) -> bool {
        self.harvest();
        self.model.peek().as_ref().is_ok_and(|m| {
            !m.has_draft()
                && files.is_none_or(|f| {
                    !f.dirty(m.document().source())
                        && !f.retained_dirty()
                        && !f.builds.busy()
                        && !f
                            .declarations
                            .as_ref()
                            .is_some_and(|s| s.dirty() || s.busy())
                })
        })
    }
}
