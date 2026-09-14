//! The active prose/source field and its local editing actions.
use crate::{editing::Writer, palette};
use freya::{code_editor::*, prelude::*};
use recite_bakeoff_authoring::{PassageKind, View, Workbench};

pub(super) fn render(writer: Writer, details: State<bool>, editor_id: AccessibilityId) -> Element {
    let actions = crate::passage_menu::render(writer, details);
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return rect().into_element();
    };
    let source = session.view() == &View::Source;
    let selected = session.selected().ok().flatten();
    let mut field = rect().width(Size::fill()).spacing(12.);
    if let Some(passage) = &selected {
        let caption = match &passage.kind {
            PassageKind::Dialogue { speaker } => {
                palette::display_name(speaker.as_deref().unwrap_or("Narration"))
            }
            PassageKind::Choice { .. } => "Player choice".into(),
        };
        field = field.child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(label().text(caption).font_size(16.))
                .child(rect().width(Size::flex(1.)))
                .child(actions),
        );
    }
    field = field.child(
        rect()
            .width(Size::fill())
            .height(Size::px(if source { 510. } else { 96. }))
            .font_family("serif")
            .font_size(20.)
            .child(if source {
                CodeEditor::new(writer.buffers.editor, editor_id)
                    .font_family("monospace")
                    .font_size(15.)
                    .gutter(true)
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
            } else {
                Input::new(writer.buffers.prose)
                    .multiline(true)
                    .width(Size::fill())
                    .height(Size::fill())
                    .theme_colors(InputColorsThemePartial {
                        background: Some(Preference::Specific(palette::reading(writer.dark))),
                        focus_background: Some(Preference::Specific(palette::reading(writer.dark))),
                        border_fill: Some(Preference::Specific(palette::rule(writer.dark))),
                        focus_border_fill: Some(Preference::Specific(palette::accent(writer.dark))),
                        ..Default::default()
                    })
                    .theme_layout(InputLayoutThemePartial {
                        corner_radius: Some(Preference::Specific(0.0.into())),
                        inner_margin: Some(Preference::Specific(12.0.into())),
                    })
                    .into_element()
            }),
    );
    if session.has_draft() {
        field = field.child(
            rect()
                .horizontal()
                .spacing(8.)
                .child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .filled()
                        .on_press(move |_| writer.perform(Workbench::apply))
                        .child("Apply draft"),
                )
                .child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .flat()
                        .on_press(move |_| {
                            writer.perform(|m| {
                                m.discard();
                                Ok(())
                            })
                        })
                        .child("Discard draft"),
                ),
        );
    }
    if let Some(passage) = selected {
        if let PassageKind::Choice { destination } = &passage.kind {
            field = field.child(
                label()
                    .text(format!(
                        "Continue to {}",
                        palette::display_name(destination.as_deref().unwrap_or("next passage"))
                    ))
                    .color(palette::muted(writer.dark)),
            );
        }
        if *details.read() {
            field = field.child(
                label()
                    .text(format!("{}@{}", passage.label, passage.id))
                    .font_family("monospace")
                    .font_size(14.),
            );
            if matches!(passage.kind, PassageKind::Choice { .. }) {
                let mut destinations = session.document().sections();
                destinations.push("END".into());
                let mut targets = rect().spacing(4.).child(label().text("Change destination"));
                for destination in destinations {
                    let caption = palette::display_name(&destination);
                    targets = targets.child(
                        Button::new()
                            .cursor_icon(CursorIcon::Pointer)
                            .flat()
                            .on_press(move |_| writer.perform(|m| m.attribute(&destination)))
                            .child(label().text(caption)),
                    );
                }
                field = field.child(targets);
            }
        }
    }
    field.into_element()
}
