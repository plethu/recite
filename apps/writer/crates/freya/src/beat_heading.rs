//! Named beat editing stays beside the script, with an explicit rename transaction.
use crate::design::Button;
use crate::design::tokens as t;
use crate::design::tokens::ProseTypography;
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
        let mut name = use_state(move || initial);
        let input_id = use_a11y();
        let title_id = use_a11y();
        use_after_side_effect(move || {
            if *open.read() {
                input_id.request_focus();
            }
        });
        let apply = move || {
            if writer.try_navigate(|m| m.rename_beat(&name.peek())).is_ok() {
                open.set(false);
                writer.inspector_focus.request_focus();
            }
        };
        let mut row = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .spacing(t::SPACE_XS);
        if *open.read() {
            let mut commit = apply;
            row = row
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .prose_font()
                        .font_size(t::title())
                        .child(
                            Input::new(name)
                                .a11y_id(input_id)
                                .width(Size::fill())
                                .on_pre_key_down(
                                    move |event: Event<KeyboardEventData>| match event.key {
                                        Key::Named(NamedKey::Enter) => {
                                            event.stop_propagation();
                                            commit();
                                            false
                                        }
                                        Key::Named(NamedKey::Escape) => {
                                            event.stop_propagation();
                                            open.set(false);
                                            title_id.request_focus();
                                            false
                                        }
                                        _ => crate::closing::text_input_key(event),
                                    },
                                ),
                        ),
                )
                .child(crate::controls::IconButton::new(
                    "Apply beat name",
                    crate::controls::Icon::Confirm,
                    apply,
                ));
        } else {
            let initial = self.id.clone();
            row = row.child(
                Button::new()
                    .flat()
                    .width(Size::flex(1.))
                    .a11y_id(title_id)
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterRenameProject,
                    ))
                    .on_press(move |_| {
                        name.set(initial.clone());
                        open.set(true);
                    })
                    .child(
                        label()
                            .text(palette::display_name(&self.id))
                            .width(Size::fill())
                            .prose_font()
                            .font_size(t::title()),
                    ),
            );
        }
        if !writer.localisation.read().active {
            row = row.child(
                Button::new()
                    .flat()
                    .named("Pin reference")
                    .on_press(move |_| writer.pin())
                    .child("Pin reference"),
            );
            if !writer.layout.standalone() {
                row = row.child(crate::controls::IconButton::new(
                    "Close beat editor",
                    crate::controls::Icon::Close,
                    move || writer.close_editor(),
                ));
            }
        }
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .child(row)
            .maybe_child((*open.read()).then(|| {
                label()
                    .text(crate::messages::text(
                        crate::messages::MsgId::WriterGuiBeatIdentifierHint,
                    ))
                    .font_size(t::small())
            }))
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.id)
    }
}
