//! Explicit reading history and one writer-pinned reference snapshot.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
#[derive(Default)]
pub(crate) struct Trail {
    document: String,
    beats: Vec<String>,
    cursor: usize,
}
impl Trail {
    pub fn visit(&mut self, document: &str, beat: &str) {
        if self.document != document {
            self.document = document.into();
            self.beats.clear();
            self.cursor = 0;
        }
        if self
            .beats
            .get(self.cursor)
            .is_some_and(|current| current == beat)
        {
            return;
        }
        self.beats.truncate(self.cursor + 1);
        self.beats.push(beat.into());
        if self.beats.len() > 128 {
            self.beats.remove(0);
        }
        self.cursor = self.beats.len() - 1;
    }
    pub fn destination(&self, document: &str, forward: bool) -> Option<String> {
        if self.document != document {
            return None;
        }
        let index = if forward {
            self.cursor.checked_add(1)?
        } else {
            self.cursor.checked_sub(1)?
        };
        self.beats.get(index).cloned()
    }
    pub fn step(&mut self, forward: bool) {
        if forward {
            self.cursor += 1;
        } else {
            self.cursor -= 1;
        }
    }
}
#[derive(Clone, PartialEq)]
pub(crate) struct Reference {
    pub document: String,
    pub block: recite_writer_model::ScriptBlock,
}
#[derive(Clone)]
pub(crate) struct ReferencePanel {
    pub writer: Writer,
}
impl PartialEq for ReferencePanel {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for ReferencePanel {
    fn render(&self) -> impl IntoElement {
        let mut state = self.writer.reference;
        let content = state.read().clone();
        rect()
            .width(Size::fill())
            .maybe_child(content.map(|reference| {
                rect()
                    .width(Size::fill())
                    .padding(t::SPACE_SM)
                    .spacing(t::SPACE_XS)
                    .child(
                        rect()
                            .horizontal()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                label()
                                    .width(Size::flex(1.))
                                    .text(format!(
                                        "Pinned snapshot · {} · {}",
                                        reference.document,
                                        palette::display_name(&reference.block.id)
                                    ))
                                    .font_size(t::TEXT_SMALL),
                            )
                            .child(
                                Button::new()
                                    .flat()
                                    .named("Unpin reference")
                                    .on_press(move |_| state.set(None))
                                    .child("×"),
                            ),
                    )
                    .child(
                        ScrollView::new()
                            .width(Size::fill())
                            .height(Size::px(200.))
                            .child(crate::branch_preview::entries(
                                &reference.block.entries,
                                self.writer.dark,
                            )),
                    )
            }))
    }
}
