//! Named beat editing stays beside the script, with an explicit rename transaction.
use crate::{editing::Writer, palette};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct BeatHeading {
    pub writer: Writer,
    pub id: String,
}
impl PartialEq for BeatHeading {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.writer.dark == other.writer.dark
    }
}
impl Component for BeatHeading {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut open = use_state(|| false);
        let initial = self.id.clone();
        let name = use_state(move || initial);
        let mut result = rect().width(Size::fill()).spacing(6.).child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .width(Size::fill())
                .child(
                    label()
                        .text(palette::display_name(&self.id))
                        .font_family("serif")
                        .font_size(24.),
                )
                .child(rect().width(Size::flex(1.)))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            let next = !*open.peek();
                            open.set(next);
                        })
                        .child("Rename"),
                )
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            writer.close_editor();
                        })
                        .child("Close"),
                ),
        );
        if *open.read() {
            result = result
                .child(
                    label()
                        .text("Beat identifier · letters, digits and underscores")
                        .font_size(12.),
                )
                .child(Input::new(name))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            writer.navigate(|m| m.rename_beat(&name.peek()));
                            if writer.message.peek().is_empty() {
                                open.set(false);
                            }
                        })
                        .child("Rename beat"),
                );
        }
        result
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.id)
    }
}
