//! Incoming prompts remain above the beat; branch destinations stay below replies.
use super::messages::{MsgId, text};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct Context {
    pub writer: Writer,
    pub beat: String,
}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.beat == other.beat && self.writer.dark == other.writer.dark
    }
}
impl Component for Context {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut expanded = use_state(|| false);
        let script = use_memo(move || {
            writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.document().script_snapshot().ok())
        });
        let links = use_memo(move || {
            script
                .read()
                .as_ref()
                .map(|blocks| recite_writer_model::scene_links(blocks))
                .unwrap_or_default()
        });
        let incoming: Vec<_> = links
            .read()
            .iter()
            .filter(|link| link.destination == self.beat && link.origin != self.beat)
            .cloned()
            .collect();
        let mut body = rect().width(Size::fill()).spacing(t::SPACE_XS);
        if !incoming.is_empty() {
            body = body.child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        let next = !*expanded.peek();
                        expanded.set(next);
                    })
                    .child(format!(
                        "{} {} ({})",
                        if *expanded.read() { "▾" } else { "▸" },
                        text(MsgId::WriterIncoming),
                        incoming.len()
                    )),
            );
            if *expanded.read() {
                for link in incoming.iter().take(32) {
                    let origin = link.origin.clone();
                    body = body.child(
                        Button::new()
                            .flat()
                            .on_press(move |_| writer.inspect(&origin))
                            .child(format!(
                                "{} → {}",
                                palette::display_name(&link.origin),
                                link.label
                            )),
                    );
                }
            }
        }
        if writer.localisation.read().catalogue.is_none() {
            return body;
        }
        let language = writer
            .localisation
            .read()
            .catalogue
            .as_ref()
            .and_then(|c| {
                c.document
                    .headers()
                    .iter()
                    .find(|h| h.key() == "Language")
                    .map(|h| h.value().to_owned())
            })
            .unwrap_or_else(|| text(MsgId::WriterTranslation));
        body.child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .width(Size::fill())
                .spacing(t::SPACE_LG)
                .child(
                    rect().width(Size::flex(1.)).child(
                        label()
                            .text(text(MsgId::WriterSource))
                            .font_size(t::TEXT_SMALL),
                    ),
                )
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(label().text(language).font_size(t::TEXT_SMALL)),
                ),
        )
    }
}
