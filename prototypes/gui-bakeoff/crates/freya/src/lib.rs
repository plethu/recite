mod editing;
mod files;
mod palette;
mod project;
mod scene;

use editing::{editor_data, perform};
use freya::{code_editor::*, prelude::*};
use recite_bakeoff_authoring::{FIXTURE, PassageKind, View, Workbench};

pub fn app() -> Element {
    workbench(false)
}

/// First file-backed workbench; comparison entry remains in-memory.
pub fn editor_app() -> Element {
    workbench(true)
}

fn workbench(file_backed: bool) -> Element {
    let mut model = use_state(|| Workbench::new(FIXTURE));
    let mut dark = use_state(|| false);
    let mut theme = use_init_theme(|| palette::theme(false));
    let message = use_state(|| "Choose a passage and start writing.".to_owned());
    let mut details = use_state(|| false);
    let mut editor = use_state(move || {
        let state = model.peek();
        editor_data(state.as_ref().map_or("", Workbench::draft), false, false)
    });
    let prose = use_state(move || {
        model
            .peek()
            .as_ref()
            .map_or("", Workbench::draft)
            .to_owned()
    });
    use_side_effect(move || {
        let draft = if model
            .peek()
            .as_ref()
            .is_ok_and(|m| matches!(m.view(), View::Source))
        {
            editor.read().rope.to_string()
        } else {
            prose.read().clone()
        };
        let differs = model
            .peek()
            .as_ref()
            .is_ok_and(|session| session.draft() != draft);
        if differs && let Ok(session) = model.write().as_mut() {
            session.set_draft(draft);
        }
    });
    let file_controls = if file_backed {
        files::controls(
            files::Buffers {
                model,
                editor,
                prose,
            },
            message,
            *dark.read(),
        )
    } else {
        rect().into_element()
    };
    let editor_id = use_a11y();
    let state = model.read();
    let session = match state.as_ref() {
        Ok(session) => session,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let source = matches!(session.view(), View::Source);
    let night = *dark.read();
    let colors = theme.read().colors.clone();
    let selected = session.selected().ok().flatten();
    let heading = selected
        .as_ref()
        .map_or("Source".to_owned(), |p| match p.kind {
            PassageKind::Dialogue { .. } => "Dialogue".to_owned(),
            PassageKind::Choice { .. } => "Player choice".to_owned(),
        });
    let mut navigation = rect()
        .width(Size::px(240.))
        .spacing(8.)
        .padding(16.)
        .child(label().text("Crossroads").font_size(22.));
    match session.document().passages() {
        Ok(passages) => {
            for passage in passages {
                let id = passage.id.clone();
                let text = match passage.kind {
                    PassageKind::Dialogue { speaker } => {
                        speaker.unwrap_or_else(|| "Narration".to_owned())
                    }
                    PassageKind::Choice { .. } => format!(
                        "Choice: {}",
                        passage.text.chars().take(28).collect::<String>()
                    ),
                };
                navigation = navigation.child(
                    Button::new()
                        .on_press(move |_| {
                            perform(model, editor, message, prose, night, |m| {
                                m.select(View::Passage(id.clone()))
                            })
                        })
                        .child(label().text(format!(
                            "{} · {}",
                            passage.section.replace('_', " "),
                            text.replace('_', " ")
                        ))),
                );
            }
        }
        Err(error) => navigation = navigation.child(label().text(error.to_string())),
    }
    let toolbar = rect()
        .horizontal()
        .spacing(8.)
        .child(label().text("recite.").font_size(28.))
        .child(
            Button::new()
                .on_press(move |_| {
                    perform(model, editor, message, prose, night, Workbench::show_script)
                })
                .child("Script"),
        )
        .child(
            Button::new()
                .on_press(move |_| {
                    perform(model, editor, message, prose, night, |m| {
                        m.select(View::Source)
                    })
                })
                .child("Source"),
        )
        .child(
            Button::new()
                .on_press(move |_| perform(model, editor, message, prose, night, Workbench::undo))
                .child("Undo"),
        )
        .child(
            Button::new()
                .on_press(move |_| perform(model, editor, message, prose, night, Workbench::redo))
                .child("Redo"),
        )
        .child(
            Button::new()
                .on_press(move |_| {
                    let next = !*dark.peek();
                    dark.set(next);
                    theme.set(palette::theme(next));
                    let mut data = editor.write();
                    data.set_theme(palette::syntax(next));
                    data.parse();
                })
                .child(if night { "Light" } else { "Dark" }),
        );
    let pending = session.has_draft();
    let mut field = rect()
        .width(Size::fill())
        .spacing(12.)
        .padding(24.)
        .maybe(source, |field| {
            field.child(label().text(heading).font_size(28.))
        });
    if let Some(passage) = selected {
        let attributes = match passage.kind {
            PassageKind::Dialogue { speaker } => {
                field = field.child(label().text(format!(
                        "Speaker: {}",
                        speaker
                            .unwrap_or_else(|| "Narration".to_owned())
                            .replace('_', " ")
                    )));
                vec!["alice".to_owned(), "cheshire_cat".to_owned()]
            }
            PassageKind::Choice { destination } => {
                field = field.child(label().text(format!(
                    "Continue to: {}",
                    destination.unwrap_or_default().replace('_', " ")
                )));
                let mut sections = session.document().sections();
                sections.push("END".to_owned());
                sections
            }
        };
        field = field.child(
            rect()
                .horizontal()
                .spacing(8.)
                .children(attributes.into_iter().map(|value| {
                    let caption = value.replace('_', " ");
                    Button::new()
                        .on_press(move |_| {
                            perform(model, editor, message, prose, night, |m| {
                                m.attribute(&value)
                            })
                        })
                        .child(label().text(caption))
                })),
        );
        field = field.child(
            Button::new()
                .on_press(move |_| {
                    details.set(!*details.peek());
                })
                .child("Line / choice details"),
        );
        if *details.read() {
            field = field.child(label().text(format!("{}@{}", passage.label, passage.id)));
        }
    }
    field = field
        .child(
            rect()
                .height(Size::px(if source { 400. } else { 110. }))
                .width(Size::fill())
                .font_family("serif")
                .font_size(20.)
                .child(if !source {
                    Input::new(prose)
                        .multiline(true)
                        .width(Size::fill())
                        .height(Size::fill())
                        .into_element()
                } else {
                    CodeEditor::new(editor, editor_id)
                        .font_family(if source { "monospace" } else { "serif" })
                        .font_size(if source { 15. } else { 20. })
                        .gutter(source)
                        .show_whitespace(false)
                        .on_pre_key_down(move |event: Event<KeyboardEventData>| match &event.key {
                            Key::Named(NamedKey::Tab) => false,
                            Key::Named(NamedKey::Escape) => {
                                editor_id.request_unfocus();
                                event.stop_propagation();
                                false
                            }
                            _ => {
                                event.stop_propagation();
                                true
                            }
                        })
                        .into_element()
                }),
        )
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .child(
                    Button::new()
                        .on_press(move |_| {
                            perform(model, editor, message, prose, night, Workbench::apply)
                        })
                        .child("Apply draft"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| {
                            perform(model, editor, message, prose, night, |m| {
                                m.discard();
                                Ok(())
                            })
                        })
                        .child("Discard draft"),
                )
                .child(
                    Button::new()
                        .enabled(!source)
                        .on_press(move |_| {
                            perform(model, editor, message, prose, night, Workbench::add_choice)
                        })
                        .child("Add choice"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| {
                            perform(
                                model,
                                editor,
                                message,
                                prose,
                                night,
                                Workbench::start_preview,
                            )
                        })
                        .child("Try scene"),
                ),
        );
    if let Some(page) = session.preview_page() {
        field = field
            .child(label().text(if session.preview_stale() {
                "Preview is out of date"
            } else {
                "Preview"
            }))
            .child(label().text(page.text.clone()).font_size(20.));
        for (index, choice) in page.choices.iter().enumerate() {
            field = field.child(
                Button::new()
                    .on_press(move |_| {
                        perform(model, editor, message, prose, night, |m| {
                            m.advance_preview(Some(index))
                        })
                    })
                    .child(label().text(choice.text.clone())),
            );
        }
        if page.choices.is_empty() && !page.ended {
            field = field.child(
                Button::new()
                    .on_press(move |_| {
                        perform(model, editor, message, prose, night, |m| {
                            m.advance_preview(None)
                        })
                    })
                    .child("Continue"),
            );
        }
    }
    for diagnostic in session.document().diagnostics() {
        field = field.child(label().text(diagnostic.message));
    }
    let field = scene::reading_surface(model, editor, message, prose, night, field.into_element());
    rect()
        .expanded()
        .background(colors.background)
        .color(colors.text_primary)
        .padding(16.)
        .spacing(12.)
        .child(toolbar)
        .child(file_controls)
        .child(
            label()
                .text(if file_backed {
                    "Recite writer · Early file-backed slice"
                } else {
                    "Crossroads · Session changes are temporary."
                })
                .color(colors.text_secondary),
        )
        .child(label().text(if pending {
            "Draft not applied · existing preview may be out of date."
        } else {
            ""
        }))
        .child(label().text(message.read().clone()))
        .child(
            ScrollView::new().height(Size::flex(1.)).child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .child(navigation)
                    .child(field),
            ),
        )
        .into_element()
}
