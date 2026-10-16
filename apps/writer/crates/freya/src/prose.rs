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
    /// The run-in margin label: a speaker, or "Replies" before the first reply.
    pub margin: Option<String>,
    /// Beside the text, such as a reply's destination.
    pub trailing: Option<Element>,
    /// Below the line, such as an expanded destination preview.
    pub below: Option<Element>,
}

impl PartialEq for ProseField {
    fn eq(&self, other: &Self) -> bool {
        self.passage == other.passage
            && self.reply_number == other.reply_number
            && self.margin == other.margin
            && self.trailing == other.trailing
            && self.below == other.below
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
        let restore_anchor = passage.id.clone();
        let mut localisation = writer.localisation;
        use_after_side_effect(move || {
            let restore = {
                let state = localisation.read();
                !state.active && state.focus.as_ref() == Some(&restore_anchor)
            };
            if restore {
                localisation.write().focus = None;
                id.request_focus();
            }
        });
        let mut hovered = use_state(|| false);
        let mut width = use_state(|| f32::INFINITY);
        let mut line_area = use_state(|| (0f32, 0f32));
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
                        .map(|p| p.text.clone())
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
        let pending_draft = selected
            && !id.is_focused()
            && writer
                .buffers
                .model
                .read()
                .as_ref()
                .is_ok_and(|m| m.has_draft());
        let speaker = matches!(passage.kind, PassageKind::Dialogue { .. });
        let wide = *width.read() >= 560. * t::ui_scale();
        let actions = crate::passage_menu::render(writer, details, &passage);
        let colors = t::colors();
        let mut buffers = writer.buffers;
        // Facts about the line, in mono, overlaid in the gap below it so hover never moves text.
        let meta = (*hovered.read() || selected).then(|| {
            rect()
                // Offset from the top by the measured line, below the text in the gap.
                .position(Position::new_absolute().left(0.).top(line_area.read().1))
                .width(Size::px(line_area.read().0))
                .height(Size::px(t::META_GAP))
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .spacing(t::SPACE_MD)
                .on_pointer_enter(move |_| hovered.set(true))
                .child(
                    t::meta(format!("{} · {}", passage.label, t::short_id(&passage.id)))
                        .font_size(11. * t::ui_scale())
                        .width(Size::flex(1.))
                        .max_lines(1),
                )
                .maybe_child(selected.then_some(actions))
        });
        let field = t::prose().child(
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
                // Lines are flat at rest; only keyboard or pointer focus draws the outline.
                .theme_colors(InputColorsThemePartial {
                    background: Some(Preference::Specific(Color::TRANSPARENT)),
                    focus_background: Some(Preference::Specific(palette::reading(writer.dark))),
                    border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
                    focus_border_fill: Some(Preference::Specific(
                        palette::accent(writer.dark) * focus_ink.get().value(),
                    )),
                    ..Default::default()
                })
                .theme_layout(InputLayoutThemePartial {
                    corner_radius: Some(Preference::Specific(t::RADIUS.into())),
                    padding: Some(Preference::Specific(t::SPACE_XS.into())),
                }),
        );
        let line = rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .spacing(t::SPACE_MD)
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_sized(move |event: Event<SizedEventData>| {
                line_area.set_if_modified((event.area.width(), event.area.height()));
            })
            .maybe_child(self.reply_number.map(|number| {
                rect()
                    .width(Size::px(22. * t::ui_scale()))
                    .padding((t::SPACE_SM, 0., 0., 0.))
                    .child(t::meta(number.to_string()))
            }))
            .child(rect().width(Size::flex(1.)).child(field))
            .maybe_child(
                wide.then(|| self.trailing.clone())
                    .flatten()
                    .map(|trailing| {
                        rect()
                            .padding((t::SPACE_XS, 0., 0., t::SPACE_SM))
                            .child(trailing)
                    }),
            )
            .maybe_child(meta);
        let caption = self
            .margin
            .as_deref()
            .map(|text| t::caps(text).color(if speaker { colors.accent } else { colors.muted }));
        // Narrow panes stack the label above the line and the destination below it.
        let (margin, stacked_caption) = if wide {
            (
                Some(
                    rect()
                        .width(Size::px(t::script_margin()))
                        .cross_align(Alignment::End)
                        .padding((t::SPACE_MD, 0., 0., 0.))
                        .maybe_child(caption),
                ),
                None,
            )
        } else {
            (None, caption)
        };
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .spacing(t::SPACE_LG)
            .on_sized(move |event: Event<SizedEventData>| {
                width.set_if_modified(event.area.width());
            })
            .maybe_child(margin)
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(t::SPACE_XS)
                    .maybe_child(stacked_caption)
                    .child(line)
                    .maybe_child(
                        (!wide)
                            .then(|| self.trailing.clone())
                            .flatten()
                            .map(|trailing| {
                                rect()
                                    .padding((0., 0., 0., 22. * t::ui_scale() + t::SPACE_MD))
                                    .child(trailing)
                            }),
                    )
                    .maybe_child(self.below.clone())
                    .maybe_child(pending_draft.then(|| {
                        rect()
                            .spacing(t::SPACE_XS)
                            .maybe_child(
                                writer.message.is_error().then(|| {
                                    label().text(writer.message.text()).font_size(t::small())
                                }),
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
                                    .child(crate::messages::text(
                                        crate::messages::MsgId::WriterDiscardDraft,
                                    )),
                            )
                    }))
                    .maybe_child(
                        (*details.read() && selected).then(|| details_content(writer, &passage)),
                    ),
            )
    }
}

fn details_content(writer: Writer, passage: &Passage) -> Element {
    let mut content = rect().width(Size::fill()).spacing(t::SPACE_XS).child(
        label()
            .text(format!("{}@{}", passage.label, passage.id))
            .font_size(t::small()),
    );
    if let PassageKind::Choice { destination } = &passage.kind {
        let destination = destination.as_deref().map_or_else(
            || "No destination".to_owned(),
            |target| {
                if target == "END" {
                    "End conversation".to_owned()
                } else {
                    crate::palette::display_name(target)
                }
            },
        );
        content = content.child(crate::route_editor::RouteEditor {
            writer,
            owner: crate::route_editor::RouteOwner::Reply(passage.id.clone()),
            heading: label()
                .text(destination)
                .font_size(t::small())
                .into_element(),
        });
    }
    content.into_element()
}
