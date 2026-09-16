//! A beat's script and explicit links to its neighbouring beats.
use crate::{controls::navigation_row, editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::{PassageKind, ScriptEntry, View, Workbench};

pub(super) fn reading_surface(writer: Writer, active: Element) -> Element {
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return active;
    };
    if session.view() == &View::Source {
        return active;
    }
    let blocks = match session.document().script() {
        Ok(blocks) => blocks,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let mut script = rect()
        .width(Size::fill())
        .max_width(Size::px(820.))
        .spacing(20.);
    for id in session.selected_block().ok().flatten().iter() {
        let Some(block) = blocks.iter().find(|block| &block.id == id) else {
            continue;
        };
        script = script.child(
            rect()
                .key(id.clone())
                .width(Size::fill())
                .spacing(6.)
                .child(crate::beat_heading::BeatHeading {
                    writer,
                    id: id.clone(),
                })
                .child(render_entries(writer, &block.id, &block.entries, true)),
        );
    }
    script = script.child(
        rect()
            .horizontal()
            .spacing(8.)
            .child(
                Button::new()
                    .flat()
                    .compact()
                    .on_press(move |_| writer.navigate(Workbench::add_line))
                    .child("Add line"),
            )
            .child(
                Button::new()
                    .flat()
                    .compact()
                    .on_press(move |_| writer.navigate(Workbench::add_choice))
                    .child("Add reply"),
            ),
    );
    script.into_element()
}

fn render_entries(
    writer: Writer,
    origin: &str,
    entries: &[ScriptEntry],
    top_level: bool,
) -> Element {
    let mut surface = rect().width(Size::fill()).spacing(6.);
    let mut in_choices = false;
    for entry in entries {
        match entry {
            ScriptEntry::Passage(passage) => {
                let choice = matches!(passage.kind, PassageKind::Choice { .. });
                if choice && !in_choices {
                    surface = surface.child(
                        label()
                            .text("Replies")
                            .font_size(13.)
                            .color(palette::muted(writer.dark)),
                    );
                }
                in_choices = choice;
                let mut row = rect()
                    .key(passage.id.clone())
                    .width(Size::fill())
                    .padding((2., 6.))
                    .border(
                        Border::new()
                            .width(BorderWidth {
                                left: 2.,
                                ..Default::default()
                            })
                            .fill(if choice {
                                palette::peach(writer.dark)
                            } else {
                                palette::rule(writer.dark)
                            }),
                    )
                    .child(crate::prose::ProseField {
                        writer,
                        passage: passage.clone(),
                    });
                if let PassageKind::Choice {
                    destination: Some(destination),
                } = &passage.kind
                {
                    row = row.child(jump(writer, origin, destination)).child(
                        crate::route_editor::RouteEditor {
                            writer,
                            owner: crate::route_editor::RouteOwner::Reply(passage.id.clone()),
                        },
                    );
                }
                surface = surface.child(row);
            }
            ScriptEntry::Jump(destination) => {
                surface = surface
                    .child(jump(writer, origin, destination))
                    .maybe_child(top_level.then(|| crate::route_editor::RouteEditor {
                        writer,
                        owner: crate::route_editor::RouteOwner::Beat(origin.into()),
                    }));
                in_choices = false;
            }
            ScriptEntry::Effect(text) => {
                surface = surface.child(
                    rect().padding((4., 10.)).child(
                        label()
                            .text(format!(
                                "Effect request · {}",
                                text.trim_start_matches('!').trim()
                            ))
                            .font_size(13.)
                            .color(palette::muted(writer.dark)),
                    ),
                );
                in_choices = false;
            }
            ScriptEntry::Group { heading, entries } => {
                surface = surface.child(
                    rect()
                        .padding((4., 12.))
                        .spacing(6.)
                        .child(label().text(heading.clone()).font_size(13.))
                        .child(render_entries(writer, origin, entries, false)),
                );
                in_choices = false;
            }
            ScriptEntry::Source(text) => {
                surface = surface.child(label().text(text.clone()).font_size(14.));
                in_choices = false;
            }
        }
    }
    surface.into_element()
}

fn jump(writer: Writer, origin: &str, target: &str) -> Element {
    if target == "END" {
        return label()
            .text("End conversation")
            .font_size(13.)
            .color(palette::muted(writer.dark))
            .into_element();
    }
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return rect().into_element();
    };
    let key = (
        session.document().key().as_str().to_owned(),
        origin.to_owned(),
        target.to_owned(),
    );
    let open = writer.expanded.read().contains(&key);
    let mut expanded = writer.expanded;
    let mut result = rect().width(Size::fill()).spacing(6.).child(navigation_row(
        format!(
            "{} {}",
            if open { "▾" } else { "▸" },
            palette::display_name(target)
        ),
        false,
        writer.dark,
        move |_| {
            let mut entries = expanded.write();
            if !entries.remove(&key) {
                entries.insert(key.clone());
            }
        },
    ));
    if open {
        let block = session
            .document()
            .script()
            .ok()
            .and_then(|blocks| blocks.into_iter().find(|block| block.id == target));
        if let Some(block) = block {
            result = result.child(crate::branch_preview::BranchPreview { writer, block });
        } else {
            result = result.child(label().text("Target is outside this scene").font_size(12.));
        }
    }
    result.into_element()
}
