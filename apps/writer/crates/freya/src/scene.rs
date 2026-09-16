//! A beat's script and explicit links to its neighbouring beats.
use crate::design::Button;
use crate::design::tokens as t;
use crate::{controls::navigation_row, editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::{View, Workbench};

pub(super) fn reading_surface(writer: Writer, active: Element) -> Element {
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return active;
    };
    if session.view() == &View::Source {
        return active;
    }
    let blocks = match session.document().script_snapshot() {
        Ok(blocks) => blocks,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let mut script = rect()
        .width(Size::fill())
        .max_width(Size::px(820.))
        .spacing(t::SPACE_MD);
    for id in session.selected_block().ok().flatten().iter() {
        let Some(index) = blocks.iter().position(|block| &block.id == id) else {
            continue;
        };
        script = script.child(
            rect()
                .key(id.clone())
                .width(Size::fill())
                .spacing(t::SPACE_XS)
                .child(crate::beat_heading::BeatHeading {
                    writer,
                    id: id.clone(),
                })
                .child(crate::script_entries::EntryPage {
                    writer,
                    blocks: blocks.clone(),
                    block: index,
                    path: Vec::new(),
                }),
        );
    }
    script = script.child(
        rect()
            .horizontal()
            .spacing(t::SPACE_XS)
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.navigate(Workbench::add_line))
                    .child("Add line"),
            )
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.navigate(Workbench::add_choice))
                    .child("Add reply"),
            ),
    );
    script.into_element()
}

pub(super) fn jump(
    writer: Writer,
    origin: &str,
    target: &str,
    owner: Option<crate::route_editor::RouteOwner>,
) -> Element {
    if target == "END" {
        let heading = label()
            .text("End conversation")
            .font_size(t::TEXT_SMALL)
            .color(palette::muted(writer.dark))
            .into_element();
        return route_heading(writer, heading, owner);
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
    let mut result = rect()
        .width(Size::fill())
        .spacing(t::SPACE_XS)
        .child(route_heading(
            writer,
            navigation_row(
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
            ),
            owner,
        ));
    if open {
        let block = session
            .document()
            .script_snapshot()
            .ok()
            .and_then(|blocks| blocks.iter().find(|block| block.id == target).cloned());
        if let Some(block) = block {
            result = result.child(crate::branch_preview::BranchPreview { writer, block });
        } else {
            result = result.child(
                label()
                    .text("Target is outside this scene")
                    .font_size(t::TEXT_SMALL),
            );
        }
    }
    result.into_element()
}

fn route_heading(
    writer: Writer,
    heading: Element,
    owner: Option<crate::route_editor::RouteOwner>,
) -> Element {
    if let Some(owner) = owner {
        crate::route_editor::RouteEditor {
            writer,
            owner,
            heading,
        }
        .into_element()
    } else {
        heading
    }
}
