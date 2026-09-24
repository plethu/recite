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
        let colors = t::colors();
        let mut collapsed = use_state(|| false);
        let mut state = self.writer.reference;
        let content = state.read().clone();
        rect()
            .width(Size::fill())
            .maybe_child(content.map(|reference| {
                rect()
                    .width(Size::fill())
                    .a11y_role(AccessibilityRole::Group)
                    .a11y_alt("Pinned reference · read only")
                    .background(colors.inset)
                    .border(
                        Border::new()
                            .width(BorderWidth {
                                top: 1.,
                                ..Default::default()
                            })
                            .fill(colors.boundary),
                    )
                    .padding(t::SPACE_MD)
                    .spacing(t::SPACE_XS)
                    .child(
                        rect()
                            .horizontal()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                Button::new()
                                    .flat()
                                    .width(Size::flex(1.))
                                    .named(crate::messages::text(
                                        crate::messages::MsgId::WriterGuiTogglePinnedReference,
                                    ))
                                    .expanded(!*collapsed.read())
                                    .on_press(move |_| {
                                        let next = !*collapsed.peek();
                                        collapsed.set(next);
                                    })
                                    .child(
                                        label()
                                            .width(Size::fill())
                                            .text(format!(
                                                "{} Pinned snapshot · {}",
                                                if *collapsed.read() { "▸" } else { "▾" },
                                                palette::display_name(&reference.block.id)
                                            ))
                                            .font_size(t::small()),
                                    ),
                            )
                            .child(
                                Button::new()
                                    .flat()
                                    .named(crate::messages::text(
                                        crate::messages::MsgId::WriterGuiUnpinReference,
                                    ))
                                    .on_press(move |_| state.set(None))
                                    .child("×"),
                            ),
                    )
                    .maybe_child((!*collapsed.read()).then(|| {
                        label()
                            .text(format!("Read only · {}", reference.document))
                            .font_size(t::small())
                            .color(colors.muted)
                    }))
                    .maybe_child((!*collapsed.read()).then(|| {
                        ScrollView::new()
                            .width(Size::fill())
                            .height(Size::px(180.))
                            .max_height(Size::window_percent(25.))
                            .child(crate::branch_preview::entries(
                                &reference.block.entries,
                                self.writer.dark,
                            ))
                    }))
            }))
    }
}
