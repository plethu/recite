//! Open text buffers own harvesting, save and scene-switch protection.
mod bookmarks;
mod drafts;
use crate::{
    editing::{Session, editor_data},
    project::ProjectFiles,
};
pub(crate) use bookmarks::Bookmarks;
pub(crate) use drafts::harvest_draft;
pub(crate) use drafts::observe_draft;
use freya::{code_editor::CodeEditorData, prelude::*};
use recite_writer_model::{View, Workbench};

/// The prose widget has no document-sized buffer while Source owns the text.
pub(crate) fn prose_draft(model: &Workbench) -> &str {
    if model.view() == &View::Source {
        ""
    } else {
        model.draft()
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Buffers {
    pub bookmarks: Bookmarks,
    pub rules: State<Option<recite_writer_model::ReplyRules>>,
    pub model: Session,
    pub editor: State<CodeEditorData>,
    pub prose: State<String>,
}

impl Buffers {
    pub(crate) fn sync_navigation(mut self, previous: &View, changed_scene: bool, dark: bool) {
        let state = self.model.peek();
        if let Ok(model) = state.as_ref() {
            if changed_scene
                || model.view() != previous
                || !model
                    .draft_snapshot()
                    .matches_rope(&self.editor.peek().rope)
            {
                self.editor.set(editor_data(
                    model.draft(),
                    model.view() == &View::Source,
                    dark,
                ));
            }
            self.prose.set_if_modified(prose_draft(model).to_owned());
        }
        self.restore();
    }
    pub(crate) fn sync(mut self, dark: bool) {
        let state = self.model.peek();
        if let Ok(model) = state.as_ref() {
            self.editor.set(editor_data(
                model.draft(),
                model.view() == &View::Source,
                dark,
            ));
            self.prose.set(prose_draft(model).into());
        }
    }

    pub(super) fn install(mut self, next: recite_writer_model::Workbench, dark: bool) {
        self.editor.set(editor_data(
            next.draft(),
            next.view() == &View::Source,
            dark,
        ));
        self.prose.set(prose_draft(&next).to_owned());
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
            harvest_draft(model, &self.editor.peek().rope, &self.prose.peek());
        }
    }

    pub(super) fn can_leave(self, files: Option<&ProjectFiles>) -> bool {
        self.harvest();
        self.model
            .peek()
            .as_ref()
            .is_ok_and(|m| !m.has_draft() && files.is_none_or(|f| f.can_leave(m)))
    }
}

#[cfg(test)]
mod tests;
