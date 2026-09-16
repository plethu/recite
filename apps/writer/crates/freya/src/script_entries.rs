//! Large beats are paged without recycling an active text editor or flattening conditions.
use crate::scene::jump;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
use recite_writer_model::{PassageKind, ScriptBlock, ScriptEntry};
use std::sync::Arc;
const PAGE_SIZE: usize = 32;
#[derive(Clone)]
pub(super) struct EntryPage {
    pub writer: Writer,
    pub blocks: Arc<[ScriptBlock]>,
    pub block: usize,
    pub path: Vec<usize>,
}
impl PartialEq for EntryPage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.blocks, &other.blocks)
            && self.block == other.block
            && self.path == other.path
            && self.writer.dark == other.writer.dark
    }
}
impl Component for EntryPage {
    fn render(&self) -> impl IntoElement {
        let mut page = use_state(|| 0usize);
        let writer = self.writer;
        let origin = self.blocks[self.block].id.as_str();
        let mut entries = self.blocks[self.block].entries.as_slice();
        for &index in &self.path {
            let Some(ScriptEntry::Group {
                entries: nested, ..
            }) = entries.get(index)
            else {
                return rect().into_element();
            };
            entries = nested;
        }
        let top_level = self.path.is_empty();
        let pages = entries.len().div_ceil(PAGE_SIZE).max(1);
        let current = (*page.read()).min(pages - 1);
        let start = current * PAGE_SIZE;
        let mut surface = rect().width(Size::fill()).spacing(t::SPACE_XS);
        let mut in_choices = false;
        let mut reply_number = entries[..start].iter().filter(|entry| matches!(entry, ScriptEntry::Passage(p) if matches!(p.kind, PassageKind::Choice { .. }))).count();
        for (index, entry) in entries.iter().enumerate().skip(start).take(PAGE_SIZE) {
            match entry {
                ScriptEntry::Passage(passage) => {
                    let choice = matches!(passage.kind, PassageKind::Choice { .. });
                    if choice && !in_choices {
                        surface = surface.child(
                            label()
                                .text("Replies")
                                .font_size(t::TEXT_SMALL)
                                .color(palette::muted(writer.dark)),
                        );
                    }
                    in_choices = choice;
                    if choice {
                        reply_number += 1;
                    }
                    let mut row = rect()
                        .key(passage.id.clone())
                        .width(Size::fill())
                        .padding((t::SPACE_XS, 0.))
                        .border(
                            Border::new()
                                .width(BorderWidth {
                                    top: if choice { 1. } else { 0. },
                                    ..Default::default()
                                })
                                .fill(palette::rule(writer.dark)),
                        )
                        .child(crate::prose::ProseField {
                            writer,
                            passage: passage.clone(),
                            reply_number: choice.then_some(reply_number),
                        });
                    if let PassageKind::Choice {
                        destination: Some(destination),
                    } = &passage.kind
                    {
                        row = row.child(jump(
                            writer,
                            origin,
                            destination,
                            Some(crate::route_editor::RouteOwner::Reply(passage.id.clone())),
                        ));
                    }
                    surface = surface.child(row);
                }
                ScriptEntry::Jump(destination) => {
                    surface = surface.child(jump(
                        writer,
                        origin,
                        destination,
                        top_level.then(|| crate::route_editor::RouteOwner::Beat(origin.into())),
                    ));
                    in_choices = false;
                }
                ScriptEntry::Effect(text) => {
                    surface = surface.child(
                        rect().padding((0., t::SPACE_XS)).child(
                            label()
                                .text(format!(
                                    "Effect request · {}",
                                    text.trim_start_matches('!').trim()
                                ))
                                .font_size(t::TEXT_SMALL)
                                .color(palette::muted(writer.dark)),
                        ),
                    );
                    in_choices = false;
                }
                ScriptEntry::Group { heading, .. } => {
                    surface = surface.child(
                        rect()
                            .padding((4., 12.))
                            .spacing(t::SPACE_XS)
                            .child(label().text(heading.clone()).font_size(t::TEXT_SMALL))
                            .child(EntryPage {
                                writer,
                                blocks: self.blocks.clone(),
                                block: self.block,
                                path: {
                                    let mut path = self.path.clone();
                                    path.push(index);
                                    path
                                },
                            }),
                    );
                    in_choices = false;
                }
                ScriptEntry::Source(text) => {
                    surface = surface.child(label().text(text.clone()).font_size(t::TEXT_BODY));
                    in_choices = false;
                }
            }
        }

        if pages > 1 {
            let mut controls = rect().horizontal().spacing(t::SPACE_SM).child(
                label()
                    .text(format!(
                        "Passages {}–{} of {}",
                        start + 1,
                        (start + PAGE_SIZE).min(entries.len()),
                        entries.len()
                    ))
                    .font_size(t::TEXT_SMALL),
            );
            for (caption, destination, enabled) in [
                ("Previous passages", current.saturating_sub(1), current > 0),
                ("Next passages", current + 1, current + 1 < pages),
            ] {
                controls = controls.child(
                    Button::new()
                        .flat()
                        .enabled(enabled)
                        .on_press(move |_| {
                            writer.navigate(|_| Ok(()));
                            if writer.message.peek().is_empty() {
                                page.set(destination);
                                writer.inspector_focus.request_focus();
                            }
                        })
                        .child(caption),
                );
            }
            surface = rect()
                .width(Size::fill())
                .spacing(t::SPACE_SM)
                .child(controls)
                .child(surface);
        }
        surface.into_element()
    }
}
