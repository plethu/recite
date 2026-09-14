//! Continuous script composition; selection uses a leading rule, not another form card.
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_bakeoff_authoring::{PassageKind, View};

pub(super) fn reading_surface(writer: Writer, active: Element) -> Element {
    let mut folded = use_state(std::collections::BTreeSet::<String>::new);
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return active;
    };
    if session.view() == &View::Source {
        return active;
    }
    let Ok(passages) = session.document().passages() else {
        return active;
    };
    let mut script = rect().width(Size::fill()).spacing(0.);
    let mut section = String::new();
    for passage in passages {
        if passage.section != section {
            section = passage.section.clone();
            let section_key = format!("{}::{section}", session.document().key().as_str());
            let is_folded = folded.read().contains(&section_key);
            script = script.child(
                rect().padding(Gaps::new(24., 0., 20., 0.)).child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .flat()
                        .on_press(move |_| {
                            let mut sections = folded.write();
                            if !sections.remove(&section_key) {
                                sections.insert(section_key.clone());
                            }
                        })
                        .child(
                            label()
                                .text(format!(
                                    "{} {}",
                                    if is_folded { "▸" } else { "▾" },
                                    palette::display_name(&section)
                                ))
                                .font_family("serif")
                                .font_size(28.),
                        ),
                ),
            );
        }
        if folded
            .read()
            .contains(&format!("{}::{section}", session.document().key().as_str()))
        {
            continue;
        }
        let selected = session.view() == &View::Passage(passage.id.clone());
        let choice = matches!(passage.kind, PassageKind::Choice { .. });
        let mark = if selected {
            palette::accent(writer.dark)
        } else if choice {
            palette::peach(writer.dark)
        } else {
            Color::TRANSPARENT
        };
        let mut passage_view = rect()
            .width(Size::fill())
            .padding(Gaps::new(16., 0., 16., 20.))
            .border(
                Border::new()
                    .width(BorderWidth {
                        left: 3.,
                        ..Default::default()
                    })
                    .fill(mark),
            )
            .spacing(10.);
        if selected {
            passage_view = passage_view.child(active.clone());
        } else {
            let caption = match &passage.kind {
                PassageKind::Dialogue { speaker } => {
                    palette::display_name(speaker.as_deref().unwrap_or("Narration"))
                }
                PassageKind::Choice { .. } => "Player choice".into(),
            };
            let id = passage.id;
            passage_view = passage_view
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::Flex)
                        .cross_align(Alignment::Center)
                        .child(label().text(caption.clone()).font_size(16.))
                        .child(rect().width(Size::flex(1.)))
                        .child(
                            Button::new()
                                .cursor_icon(CursorIcon::Pointer)
                                .flat()
                                .on_press(move |_| {
                                    writer.perform(|m| m.select(View::Passage(id.clone())));
                                })
                                .child(label().text(format!("Edit {caption}"))),
                        ),
                )
                .child(
                    label()
                        .text(passage.text)
                        .font_family("serif")
                        .font_size(20.),
                );
            if let PassageKind::Choice { destination } = passage.kind {
                passage_view = passage_view.child(
                    label()
                        .text(format!(
                            "Continue to {}",
                            palette::display_name(destination.as_deref().unwrap_or("next passage"))
                        ))
                        .color(palette::muted(writer.dark))
                        .font_size(14.),
                );
            }
        }
        script = script.child(passage_view).child(
            rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(palette::rule(writer.dark)),
        );
    }
    script.into_element()
}
