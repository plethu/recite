//! Empty and welcome states: a dashed plum frame with a halftone tint, the registration mark,
//! a display title and one line of instruction. Nothing here animates.
use super::tokens::{self as t, PrintTypography};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct EmptyState {
    pub title: String,
    pub instruction: String,
    pub actions: Vec<Element>,
    pub min_height: f32,
}

impl Component for EmptyState {
    fn render(&self) -> impl IntoElement {
        let p = t::colors();
        rect()
            .width(Size::fill())
            .min_height(Size::px(self.min_height))
            .corner_radius(t::RADIUS)
            .overflow(Overflow::Clip)
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_SM)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(self.title.clone())
            .child(super::material::halftone(
                p.teal,
                t::HALFTONE_FIELD_ALPHA,
                1.2,
                8.,
                true,
            ))
            .child(super::material::dashed_frame(
                p.accent,
                t::STROKE_WIDTH,
                t::RADIUS,
            ))
            .child(super::material::registration(p.accent, 18.))
            .child(
                label()
                    .text(self.title.clone())
                    .display_font()
                    .font_weight(FontWeight::SEMI_BOLD)
                    .font_size(t::title()),
            )
            .child(label().text(self.instruction.clone()).color(p.muted))
            .maybe_child((!self.actions.is_empty()).then(|| {
                rect()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .spacing(t::SPACE_SM)
                    .padding((t::SPACE_SM, 0., 0., 0.))
                    .children(self.actions.clone())
            }))
    }
}
