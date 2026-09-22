//! A drawn checkbox shares the button's hit target and activation policy.
use super::{Button, tokens as t};
use freya::prelude::*;

pub(crate) fn checkbox(
    id: AccessibilityId,
    caption: impl Into<String>,
    checked: bool,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    Checkbox {
        id,
        caption: caption.into(),
        checked,
        action: EventHandler::new(action),
    }
    .into_element()
}
#[derive(Clone, PartialEq)]
struct Checkbox {
    id: AccessibilityId,
    caption: String,
    checked: bool,
    action: EventHandler<Event<PressEventData>>,
}
impl Component for Checkbox {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        Button::new()
            .flat()
            .a11y_id(self.id)
            .named(self.caption.clone())
            .checkable(self.checked)
            .width(Size::fill())
            .on_press(self.action.clone())
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .spacing(t::SPACE_SM)
                    .cross_align(Alignment::Center)
                    .child(
                        rect()
                            .width(Size::px(16.))
                            .height(Size::px(16.))
                            .corner_radius(2.)
                            .border(Border::new().width(1.).fill(colors.accent))
                            .background(if self.checked {
                                colors.accent
                            } else {
                                Color::TRANSPARENT
                            })
                            .center()
                            .maybe_child(self.checked.then(|| {
                                label()
                                    .text("✓")
                                    .font_size(t::small())
                                    .color(colors.on_accent)
                            })),
                    )
                    .child(label().text(self.caption.clone())),
            )
    }
}
