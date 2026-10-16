//! Mirror text changes without recapturing Source for cursor or syntax updates.
use super::Session;
use freya::{
    code_editor::{CodeEditorData, Rope},
    prelude::*,
};
use recite_writer_model::{DraftSnapshot, View, Workbench};

pub(crate) fn harvest_draft(model: &mut Workbench, source: &Rope, prose: &str) {
    if model.view() == &View::Source {
        if !model.draft_snapshot().matches_rope(source) {
            model.set_draft_snapshot(DraftSnapshot::from_rope(source.clone()));
        }
    } else if model.draft() != prose {
        model.set_draft(prose.into());
    }
}

#[derive(Default)]
struct SourceDraft {
    previous: Option<Rope>,
}
impl SourceDraft {
    fn capture<'a>(
        &mut self,
        source: &Rope,
        current: impl FnOnce() -> &'a DraftSnapshot,
    ) -> Option<DraftSnapshot> {
        if self
            .previous
            .as_ref()
            .is_some_and(|old| old.is_instance(source))
        {
            return None;
        }
        self.previous = Some(source.clone());
        (!current().matches_rope(source)).then(|| DraftSnapshot::from_rope(source.clone()))
    }
}

pub(crate) fn observe_draft(
    mut model: Session,
    editor: State<CodeEditorData>,
    prose: State<String>,
) {
    let mut source = use_state(SourceDraft::default);
    use_side_effect(move || {
        let draft = {
            let state = model.read();
            let Ok(session) = state.as_ref() else {
                return;
            };
            if session.view() == &View::Source {
                source
                    .write()
                    .capture(&editor.read().rope, || session.draft_snapshot())
            } else {
                source.write().previous = None;
                let text = prose.read();
                (session.draft() != text.as_str()).then(|| DraftSnapshot::from(text.clone()))
            }
        };
        if let Some(draft) = draft
            && let Ok(session) = model.write().as_mut()
        {
            session.set_draft_snapshot(draft);
        }
    });
}

#[cfg(test)]
mod tests;
