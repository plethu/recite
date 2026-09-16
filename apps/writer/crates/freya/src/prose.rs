//! Persistent text fields: focus changes decoration, never the text's geometry.
use crate::design::Button;
use crate::design::tokens as t;
use crate::{editing::Writer, palette};
use freya::{animation::use_animation, prelude::*};
use recite_writer_model::{Passage, PassageKind, View, Workbench};

#[derive(Clone)]
pub(super) struct ProseField {
    pub writer: Writer,
    pub passage: Passage,
    pub reply_number: Option<usize>,
}

impl PartialEq for ProseField {
    fn eq(&self, other: &Self) -> bool {
        self.passage == other.passage
            && self.reply_number == other.reply_number
            && self.writer.dark == other.writer.dark
    }
}

impl Component for ProseField {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let reduced = writer.preferences.read().config.writer.reduced_motion;
        let passage = self.passage.clone();
        let id = use_a11y();
        let details = use_state(|| false);
        let mut hovered = use_state(|| false);
        let initial = passage.text.clone();
        let mut text = use_state(move || initial);
        // Colour feedback only: no text, panel, or camera movement is animated.
        let mut focus_ink = use_animation(move |_| t::transition(0.35, 1., reduced));
        let mut was_focused = use_state(|| false);
        let target = View::Passage(passage.id.clone());
        let focus_target = target.clone();
        use_side_effect(move || {
            let focused = id.is_focused();
            if focused && !*was_focused.peek() {
                focus_ink.start();
                writer.navigate(|m| m.select(focus_target.clone()));
            } else if !focused && *was_focused.peek() {
                let selected = writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .is_ok_and(|m| m.view() == &focus_target && m.has_draft());
                if selected {
                    writer.perform(Workbench::apply);
                }
            }
            was_focused.set_if_modified(focused);
        });
        let sync_target = target.clone();
        let passage_id = passage.id.clone();
        use_side_effect(move || {
            let state = writer.buffers.model.read();
            if let Ok(session) = state.as_ref() {
                let current = if session.view() == &sync_target {
                    Some(session.draft().to_owned())
                } else {
                    session
                        .document()
                        .find_passage(&passage_id)
                        .ok()
                        .flatten()
                        .map(|p| p.text)
                };
                if let Some(current) = current {
                    text.set_if_modified(current);
                }
            }
        });
        let selected = writer
            .buffers
            .model
            .read()
            .as_ref()
            .is_ok_and(|m| m.view() == &target);
        let pending_error = selected
            && !id.is_focused()
            && writer
                .buffers
                .model
                .read()
                .as_ref()
                .is_ok_and(|m| m.has_draft())
            && !writer.message.read().is_empty();
        let caption = match &passage.kind {
            PassageKind::Dialogue { speaker } => {
                palette::display_name(speaker.as_deref().unwrap_or("Narration"))
            }
            PassageKind::Choice { .. } => self
                .reply_number
                .map_or_else(|| "Reply".into(), |number| format!("Reply {number}")),
        };
        let actions = crate::passage_menu::render(writer, details);
        let mut buffers = writer.buffers;
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .child(
                rect()
                    .height(Size::px(t::PROSE_META_HEIGHT))
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .child(
                        label()
                            .text(caption)
                            .font_size(t::TEXT_SMALL)
                            .color(palette::muted(writer.dark)),
                    )
                    .child(rect().width(Size::flex(1.)))
                    .maybe_child(selected.then_some(actions)),
            )
            .child(
                rect()
                    .font_family("serif")
                    .font_size(t::TEXT_HEADING)
                    .child(
                        Input::new(text)
                            .a11y_id(id)
                            .multiline(true)
                            .width(Size::fill())
                            .height(Size::Inner)
                            .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                                if event.key == Key::Named(NamedKey::Escape) {
                                    event.stop_propagation();
                                    writer.close_editor();
                                    false
                                } else {
                                    crate::closing::text_input_key(event)
                                }
                            })
                            .on_validate(move |value: InputValidator| {
                                let selected = buffers
                                    .model
                                    .peek()
                                    .as_ref()
                                    .is_ok_and(|m| m.view() == &target);
                                if !selected {
                                    writer.navigate(|m| m.select(target.clone()));
                                }
                                if let Ok(session) = buffers.model.write().as_mut() {
                                    if session.view() == &target {
                                        let draft = value.text().clone();
                                        session.set_draft(draft.clone());
                                        buffers.prose.set_if_modified(draft);
                                    } else {
                                        value.set_valid(false);
                                    }
                                }
                            })
                            .theme_colors(InputColorsThemePartial {
                                background: Some(Preference::Specific(Color::TRANSPARENT)),
                                focus_background: Some(Preference::Specific(palette::reading(
                                    writer.dark,
                                ))),
                                border_fill: Some(Preference::Specific(if *hovered.read() {
                                    palette::rule(writer.dark)
                                } else {
                                    Color::TRANSPARENT
                                })),
                                focus_border_fill: Some(Preference::Specific(
                                    palette::accent(writer.dark) * focus_ink.get().value(),
                                )),
                                ..Default::default()
                            })
                            .theme_layout(InputLayoutThemePartial {
                                corner_radius: Some(Preference::Specific(3.0.into())),
                                inner_margin: Some(Preference::Specific(t::SPACE_XS.into())),
                            }),
                    ),
            )
            .maybe_child(pending_error.then(|| {
                rect()
                    .spacing(t::SPACE_XS)
                    .child(
                        label()
                            .text(writer.message.read().clone())
                            .font_size(t::TEXT_SMALL),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| {
                                writer.perform(|m| {
                                    m.discard();
                                    Ok(())
                                })
                            })
                            .child("Discard draft"),
                    )
            }))
            .maybe_child((*details.read() && selected).then(|| details_content(writer, &passage)))
    }
}

fn details_content(writer: Writer, passage: &Passage) -> Element {
    let mut content = rect().width(Size::fill()).spacing(t::SPACE_XS).child(
        label()
            .text(format!("{}@{}", passage.label, passage.id))
            .font_size(t::TEXT_SMALL),
    );
    if matches!(passage.kind, PassageKind::Choice { .. }) {
        let mut destinations = writer
            .buffers
            .model
            .peek()
            .as_ref()
            .map(|m| m.document().sections())
            .unwrap_or_default();
        destinations.push("END".into());
        content = content.child(label().text("Change destination"));
        for destination in destinations {
            let caption = palette::display_name(&destination);
            content = content.child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.navigate(|m| m.attribute(&destination)))
                    .child(caption),
            );
        }
    }
    content.into_element()
}
