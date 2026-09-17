//! A dialog's button and submit shortcut invoke the same guarded action.
use super::{Button, keyboard, tokens as t};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct DialogAction {
    pub id: AccessibilityId,
    pub caption: String,
    pub enabled: bool,
    pub action: EventHandler<()>,
}
impl DialogAction {
    pub fn run(&self) {
        if self.enabled {
            self.action.call(());
        }
    }
    pub fn button(&self) -> Button {
        let action = self.clone();
        Button::new()
            .filled()
            .a11y_id(self.id)
            .named(self.caption.clone())
            .enabled(self.enabled)
            .on_press(move |_| action.run())
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_MD)
                    .cross_align(Alignment::Center)
                    .child(label().text(self.caption.clone()))
                    .child(
                        label()
                            .text(keyboard::submit_hint())
                            .font_size(t::TEXT_SMALL),
                    ),
            )
    }
}
