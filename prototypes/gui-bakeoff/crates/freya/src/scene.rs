use freya::{code_editor::CodeEditorData, prelude::*};
use recite_bakeoff_authoring::{PassageKind, View};

use crate::editing::{Session, perform};

pub fn reading_surface(
    model: Session,
    editor: State<CodeEditorData>,
    message: State<String>,
    prose: State<String>,
    dark: bool,
    active: Element,
) -> Element {
    let state = model.peek();
    let Ok(session) = state.as_ref() else {
        return active;
    };
    if matches!(session.view(), View::Source) {
        return active;
    }
    let Ok(passages) = session.document().passages() else {
        return active;
    };
    let mut script = rect().width(Size::fill()).padding(24.).spacing(20.);
    let mut section = String::new();
    for passage in passages {
        if passage.section != section {
            section = passage.section.clone();
            script = script.child(
                label()
                    .text(section.replace('_', " "))
                    .font_family("serif")
                    .font_size(28.),
            );
        }
        if session.view() == &View::Passage(passage.id.clone()) {
            script = script.child(active.clone());
            continue;
        }
        let (caption, choice) = match &passage.kind {
            PassageKind::Dialogue { speaker } => (
                speaker.as_deref().unwrap_or("Narration").replace('_', " "),
                false,
            ),
            PassageKind::Choice { .. } => ("Player choice".to_owned(), true),
        };
        let background = match (dark, choice) {
            (false, false) => Color::from_rgb(255, 253, 250),
            (true, false) => Color::from_rgb(41, 42, 45),
            (false, true) => Color::from_rgb(241, 212, 191),
            (true, true) => Color::from_rgb(65, 51, 43),
        };
        let mut card = rect()
            .width(Size::fill())
            .padding(20.)
            .spacing(12.)
            .background(background)
            .child(label().text(caption.clone()))
            .child(
                label()
                    .text(passage.text)
                    .font_family("serif")
                    .font_size(20.),
            );
        if let PassageKind::Choice { destination } = passage.kind {
            card = card.child(label().text(format!(
                    "Continue to {}",
                    destination
                        .as_deref()
                        .unwrap_or("next passage")
                        .replace('_', " ")
                )));
        }
        let id = passage.id;
        script = script.child(
            card.child(
                Button::new()
                    .on_press(move |_| {
                        perform(model, editor, message, prose, dark, |m| {
                            m.select(View::Passage(id.clone()))
                        });
                    })
                    .child(label().text(format!("Edit {caption}"))),
            ),
        );
    }
    script.into_element()
}
