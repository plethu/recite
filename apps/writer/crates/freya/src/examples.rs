//! Temporary example sessions retain edits, drafts, and undo when switching scenes.
use crate::{
    editing::{Writer, editor_data},
    palette,
};
use freya::prelude::*;
use recite_writer_model::{View, WRITER_EXAMPLES};

pub(super) fn navigation(writer: Writer, mut message: State<String>, dark: bool) -> Element {
    let mut buffers = writer.buffers;
    let mut parked = use_state(std::collections::BTreeMap::new);
    let mut selected = use_state(|| 0_usize);
    let mut scenes = Vec::new();
    for (index, example) in WRITER_EXAMPLES.iter().enumerate() {
        scenes.push(crate::scene_navigation::SceneBranch {
            caption: palette::display_name(example.name),
            active: *selected.read() == index,
            open: EventHandler::new(move |()| {
                let previous = *selected.peek();
                if previous == index {
                    return;
                }
                buffers.harvest();
                let next = parked
                    .write()
                    .remove(&index)
                    .unwrap_or_else(|| example.open());
                let mut next = match next {
                    Ok(next) => Ok(next),
                    Err(error) => {
                        message.set(error.to_string());
                        return;
                    }
                };
                std::mem::swap(&mut *buffers.model.write(), &mut next);
                parked.write().insert(previous, next);
                selected.set(index);
                if let Ok(session) = buffers.model.peek().as_ref() {
                    buffers.editor.set(editor_data(
                        session.draft(),
                        session.view() == &View::Source,
                        dark,
                    ));
                    buffers.prose.set(session.draft().to_owned());
                }
                message.set(String::new());
            }),
        });
    }
    crate::scene_navigation::SceneNavigation { writer, scenes }.into_element()
}
