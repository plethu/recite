//! Enter at final layout size; only position and opacity change during motion.
use super::tokens as t;
use freya::{
    animation::{OnCreation, use_animation},
    prelude::*,
};

#[derive(Clone, PartialEq)]
pub(crate) struct Reveal {
    pub content: Element,
    pub reduced_motion: bool,
}
impl Component for Reveal {
    fn render(&self) -> impl IntoElement {
        let reduced = self.reduced_motion;
        let progress = use_animation(move |config| {
            config.on_creation(OnCreation::Run);
            t::transition(0., 1., reduced)
        });
        let amount = progress.get().value();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .offset_x(-t::SPACE_MD * (1. - amount))
                    .opacity(amount)
                    .child(self.content.clone()),
            )
    }
}
