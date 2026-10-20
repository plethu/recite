//! Temporary example sessions retain edits, drafts, and undo when switching scenes.
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::WRITER_EXAMPLES;

pub(super) fn navigation(writer: Writer, mut message: crate::feedback::Feedback) -> Element {
    let mut scenes = Vec::new();
    for (index, example) in WRITER_EXAMPLES.iter().enumerate() {
        scenes.push(crate::scene_navigation::SceneBranch {
            caption: palette::display_name(example.name),
            active: writer
                .buffers
                .model
                .read()
                .as_ref()
                .is_ok_and(|m| m.document().key().as_str() == example.name),
            open: EventHandler::new(move |()| match select(writer, index) {
                Err(error) => message.error(error),
                Ok(()) => {
                    let _ = writer.resume_scene();
                }
            }),
        });
    }
    crate::scene_navigation::SceneNavigation { writer, scenes }.into_element()
}

/// Swap complete temporary sessions so browser history never discards drafts.
pub(super) fn select(writer: Writer, index: usize) -> Result<(), String> {
    select_at(writer, index, |_| Ok(()))
}

pub(super) fn select_at(
    mut writer: Writer,
    index: usize,
    select: impl FnOnce(
        &mut recite_writer_model::Workbench,
    ) -> Result<(), recite_writer_model::WorkbenchError>,
) -> Result<(), String> {
    let example = WRITER_EXAMPLES
        .get(index)
        .ok_or("The selected example is unavailable.")?;
    if writer
        .buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|model| model.document().key().as_str() == example.name)
    {
        return Ok(());
    }
    let previous_scene = writer.scene_key();
    writer.remember_scene();
    writer.buffers.harvest();
    {
        let mut model = writer.buffers.model.write();
        let model = model.as_mut().map_err(|e| e.to_string())?;
        writer
            .examples
            .write()
            .switch(model, index, select)
            .map_err(|e| e.to_string())?;
    }
    let state = writer.buffers.model.peek();
    if let Ok(session) = state.as_ref() {
        writer.buffers.editor.set(crate::editing::editor_data(
            session.draft(),
            matches!(session.view(), recite_writer_model::View::Source),
            writer.dark,
        ));
        writer
            .buffers
            .prose
            .set(crate::buffers::prose_draft(session).to_owned());
    }
    drop(state);
    writer.switch_rules(previous_scene);
    writer.buffers.restore();
    writer.message.clear();
    Ok(())
}
