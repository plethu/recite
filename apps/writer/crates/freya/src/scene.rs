//! A beat's script and explicit links to its neighbouring beats.
use crate::design::Button;
use crate::design::tokens as t;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::{View, Workbench};

/// An example scene that declares no beats yet. Its Script and Map show their empty states;
/// files opened elsewhere keep opening in Source, where the writer can type the first beat.
pub(super) fn has_no_beats(session: &recite_writer_model::Workbench) -> bool {
    try_consume_context::<crate::AppMode>() == Some(crate::AppMode::Examples)
        && session
            .document()
            .script_snapshot()
            .is_ok_and(|blocks| blocks.is_empty())
}

pub(super) fn reading_surface(writer: Writer, active: Element) -> Element {
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return active;
    };
    if session.view() == &View::Source
        && !writer.localisation.read().active
        && !(has_no_beats(session)
            && *writer.layout.view.read() != recite_config::WriterView::Source)
    {
        return active;
    }
    let blocks = match session.document().script_snapshot() {
        Ok(blocks) => blocks,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let mut script = rect()
        .width(Size::fill())
        .max_width(Size::px(if writer.localisation.read().translating() {
            1320.
        } else {
            820.
        }))
        .spacing(t::SPACE_MD);
    let open = session.selected_block().ok().flatten();
    if open.is_none() && !writer.localisation.read().active {
        script = script.child(crate::design::EmptyState {
            title: if blocks.is_empty() {
                "This scene is empty".into()
            } else {
                "No beat open".into()
            },
            instruction: if blocks.is_empty() {
                "Add a beat to start writing.".into()
            } else {
                "Choose a beat in the sidebar or on the map.".into()
            },
            actions: if blocks.is_empty() {
                vec![
                    Button::new()
                        .stamp()
                        .on_press(move |_| writer.navigate(Workbench::add_beat))
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiAddBeat,
                        ))
                        .into_element(),
                ]
            } else {
                Vec::new()
            },
            min_height: 240.,
        });
    }
    for id in open.iter() {
        let Some(index) = blocks.iter().position(|block| &block.id == id) else {
            continue;
        };
        script =
            script.child(
                rect()
                    .key(id.clone())
                    .width(Size::fill())
                    .spacing(t::SPACE_XS)
                    .maybe_child((!writer.localisation.read().active).then(|| {
                        crate::beat_heading::BeatHeading {
                            writer,
                            id: id.clone(),
                        }
                    }))
                    .maybe_child(
                        (!writer.localisation.read().active)
                            .then(|| slug(session.document().key().as_str(), &blocks[index])),
                    )
                    .maybe_child(blocks[index].entries.is_empty().then(|| {
                        crate::design::EmptyState {
                            title: "This beat is empty".into(),
                            instruction: "Add a line to start writing it.".into(),
                            actions: Vec::new(),
                            min_height: 200.,
                        }
                    }))
                    .child(crate::script_entries::EntryPage {
                        writer,
                        blocks: blocks.clone(),
                        block: index,
                        path: Vec::new(),
                    })
                    .maybe_child(
                        (!writer.localisation.read().active).then(|| end_mark(&blocks[index])),
                    ),
            );
    }
    if !writer.localisation.read().active {
        if open.is_some() {
            script = script.child(AddActions { writer });
        }
        if let Some(index) = open
            .as_ref()
            .and_then(|id| blocks.iter().position(|block| &block.id == id))
        {
            script = script.child(folio(
                session.document().key().as_str(),
                &blocks[index].id,
                index,
                blocks.len(),
            ));
        }
    }
    script.into_element()
}

/// "Add line" and "Add reply" align with the text column, past the margin when there is one.
#[derive(Clone, Copy)]
struct AddActions {
    writer: Writer,
}
impl PartialEq for AddActions {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for AddActions {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut width = use_state(|| f32::INFINITY);
        let indent = if *width.read() >= 560. * t::ui_scale() {
            t::script_margin() + t::SPACE_LG
        } else {
            0.
        };
        rect()
            .width(Size::fill())
            .horizontal()
            .spacing(t::SPACE_XS)
            .padding((0., 0., 0., indent))
            .on_sized(move |event: Event<SizedEventData>| {
                width.set_if_modified(event.area.width());
            })
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.navigate(Workbench::add_line))
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiAddLine,
                    )),
            )
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.navigate(Workbench::add_choice))
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiAddReply,
                    )),
            )
    }
}

/// Where a beat's last line ends: `◆ END ◆`, or the beat it diverts to, between dotted rules.
fn end_mark(block: &recite_writer_model::ScriptBlock) -> Element {
    let p = t::colors();
    let text = match block.entries.last() {
        Some(recite_writer_model::ScriptEntry::Jump(target)) if target != "END" => {
            format!("◆ → {} ◆", palette::display_name(target))
        }
        _ => "◆ END ◆".to_owned(),
    };
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_MD)
        .padding((t::SPACE_LG, 0.))
        .child(crate::design::material::dotted_rule(p.rule, "before"))
        .child(t::meta(text).font_size(11. * t::ui_scale()))
        .child(crate::design::material::dotted_rule(p.rule, "after"))
        .into_element()
}

/// The foot of the Script column: a plum rule over the file and beat, and the beat's place.
fn folio(key: &str, block: &str, index: usize, count: usize) -> Element {
    let p = t::colors();
    let file = if key.ends_with(".recite") {
        key.to_owned()
    } else {
        format!("{key}.recite")
    };
    rect()
        .width(Size::fill())
        .padding((t::SPACE_XL, 0., t::SPACE_LG, 0.))
        .spacing(t::SPACE_SM)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(t::STROKE_WIDTH))
                .background(p.accent),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::Flex)
                .child(t::meta(format!("{file} · {block}")).width(Size::flex(1.)))
                .child(t::meta(format!("beat {} / {count}", index + 1))),
        )
        .into_element()
}

/// The mono slug line under a beat title: file, block, flags, reply count.
fn slug(key: &str, block: &recite_writer_model::ScriptBlock) -> Element {
    use t::Slug::{Flag, Meta};
    let file = if key.ends_with(".recite") {
        key.to_owned()
    } else {
        format!("{key}.recite")
    };
    let replies = block
        .entries
        .iter()
        .filter(|entry| {
            matches!(entry, recite_writer_model::ScriptEntry::Passage(p)
                if matches!(p.kind, recite_writer_model::PassageKind::Choice { .. }))
        })
        .count();
    let mut parts = vec![Meta(file), Meta(format!(":: {}", block.id))];
    if block.is_default {
        parts.push(Flag("default".into()));
    }
    parts.push(Meta(match replies {
        0 => "no replies".into(),
        1 => "1 reply".into(),
        n => format!("{n} replies"),
    }));
    rect()
        .padding((0., 0., t::SPACE_LG, t::SPACE_XS))
        .child(t::slug(parts))
        .into_element()
}

pub(super) fn jump(
    writer: Writer,
    origin: &str,
    target: &str,
    owner: Option<crate::route_editor::RouteOwner>,
) -> Element {
    let (heading, preview) = jump_parts(writer, origin, target, owner);
    rect()
        .width(Size::fill())
        .spacing(t::SPACE_XS)
        .child(heading)
        .maybe_child(preview)
        .into_element()
}

/// A route's heading (the destination and its Change action) and, when expanded, its preview.
/// Script replies place the heading beside the reply and the preview below it.
pub(super) fn jump_parts(
    writer: Writer,
    origin: &str,
    target: &str,
    owner: Option<crate::route_editor::RouteOwner>,
) -> (Element, Option<Element>) {
    if target == "END" {
        let heading = label()
            .text(crate::messages::text(
                crate::messages::MsgId::WriterGuiEndConversation,
            ))
            .font_size(t::small())
            .color(palette::muted(writer.dark))
            .into_element();
        return (route_heading(writer, heading, owner), None);
    }
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return (rect().into_element(), None);
    };
    let key = (
        session.document().key().as_str().to_owned(),
        origin.to_owned(),
        target.to_owned(),
    );
    let open = writer.expanded.read().contains(&key);
    let mut expanded = writer.expanded;
    let heading = route_heading(
        writer,
        Button::new()
            .destination()
            .expanded(open)
            .child(format!(
                "{} {}",
                if open { "▾" } else { "→" },
                palette::display_name(target)
            ))
            .on_press(move |_| {
                let mut entries = expanded.write();
                if !entries.remove(&key) {
                    entries.insert(key.clone());
                }
            })
            .into_element(),
        owner,
    );
    if !open {
        return (heading, None);
    }
    let block = session
        .document()
        .script_snapshot()
        .ok()
        .and_then(|blocks| blocks.iter().find(|block| block.id == target).cloned());
    let preview = if let Some(block) = block {
        crate::branch_preview::BranchPreview { writer, block }.into_element()
    } else {
        label()
            .text(crate::messages::text(
                crate::messages::MsgId::WriterGuiTargetIsOutsideThisScene,
            ))
            .font_size(t::small())
            .into_element()
    };
    (heading, Some(preview))
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
