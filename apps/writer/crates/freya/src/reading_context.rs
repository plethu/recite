//! Explicit reading history and one writer-pinned reference snapshot.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
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
