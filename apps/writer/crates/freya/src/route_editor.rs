//! Destination editing beside the reply or continuation it changes.
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::View;

#[derive(Clone, PartialEq)]
pub(super) enum RouteOwner {
    Reply(String),
    Beat(String),
}

#[derive(Clone)]
pub(super) struct RouteEditor {
    pub writer: Writer,
    pub owner: RouteOwner,
}
impl PartialEq for RouteEditor {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.writer.dark == other.writer.dark
    }
}
impl Component for RouteEditor {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut open = use_state(|| false);
        let mut result = rect().width(Size::fill()).child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| {
                    let next = !*open.peek();
                    open.set(next);
                })
                .child(
                    label()
                        .text(if *open.read() {
                            "Cancel connection"
                        } else {
                            "Change destination…"
                        })
                        .font_size(12.),
                ),
        );
        if *open.read() {
            let destinations = writer
                .buffers
                .model
                .read()
                .as_ref()
                .map(|m| m.document().sections())
                .unwrap_or_default();
            for target in destinations
                .into_iter()
                .chain(std::iter::once("END".into()))
            {
                let owner = self.owner.clone();
                result = result.child(
                    Button::new()
                        .flat()
                        .compact()
                        .child(palette::display_name(&target))
                        .on_press(move |_| {
                            writer.navigate(|m| match &owner {
                                RouteOwner::Reply(id) => {
                                    m.select(View::Passage(id.clone()))?;
                                    m.attribute(&target)
                                }
                                RouteOwner::Beat(block) => {
                                    m.select(View::Block(block.clone()))?;
                                    m.set_continuation(&target)
                                }
                            });
                            if writer.message.peek().is_empty() {
                                open.set(false);
                            }
                        }),
                );
            }
        }
        result
    }
}
