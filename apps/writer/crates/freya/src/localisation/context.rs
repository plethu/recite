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
                    .expanded(*expanded.read())
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
                let mut prompts = rect().width(Size::fill()).spacing(t::SPACE_XS);
                for link in incoming.iter().take(32) {
                    let origin = link.origin.clone();
                    prompts = prompts.child(
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
                body = body.child(
                    ScrollView::new()
                        .width(Size::fill())
                        .height(Size::px(
                            (incoming.len().min(4) as f32) * (t::control_height() + t::SPACE_XS),
                        ))
                        .max_height(Size::window_percent(20.))
                        .child(prompts),
                );
            }
        }
        body
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.beat)
    }
}
