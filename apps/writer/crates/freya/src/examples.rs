//! Temporary example sessions retain edits, drafts, and undo when switching scenes.
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::WRITER_EXAMPLES;

pub(super) fn navigation(writer: Writer, mut message: State<String>) -> Element {
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
                Err(error) => message.set(error),
                Ok(()) => writer.scene_opened(),
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
    let example = &WRITER_EXAMPLES[index];
    let current = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .map_err(|e| e.to_string())?
        .document()
        .key()
        .to_string();
    if current == example.name {
        return Ok(());
    }
    writer.buffers.harvest();
    let next = {
        let mut parked = writer.examples.write();
        if let Some(next) = parked.get_mut(example.name) {
            select(next).map_err(|e| e.to_string())?;
            parked
                .remove(example.name)
                .ok_or("Example session disappeared.")?
        } else {
            let mut next = example.open().map_err(|e| e.to_string())?;
            select(&mut next).map_err(|e| e.to_string())?;
            next
        }
    };
    let previous = std::mem::replace(&mut *writer.buffers.model.write(), Ok(next));
    if let Ok(previous) = previous {
        writer.examples.write().insert(current, previous);
    }
    let state = writer.buffers.model.peek();
    if let Ok(session) = state.as_ref() {
        writer.buffers.editor.set(crate::editing::editor_data(
            session.draft(),
            matches!(session.view(), recite_writer_model::View::Source),
            writer.dark,
        ));
        writer.buffers.prose.set(session.draft().to_owned());
    }
    writer.message.set(String::new());
    Ok(())
}
