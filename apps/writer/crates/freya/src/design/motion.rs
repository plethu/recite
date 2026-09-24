//! Shared surface feedback retargets from the current frame; presses are immediate.
use freya::{
    animation::{AnimColor, Ease, Function, OnChange, use_animation_with_dependencies},
    prelude::*,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
pub(crate) struct ReducedMotion(pub State<bool>);

pub(super) fn surface(target: Color, pressed: bool) -> Color {
    let reduced = use_try_consume::<ReducedMotion>().is_none_or(|motion| *motion.0.read());
    let current = use_hook(|| Rc::new(Cell::new(target)));
    let previous = current.clone();
    let animation = use_animation_with_dependencies(
        &(target, pressed, reduced),
        move |config, (target, pressed, reduced)| {
            config.on_change(OnChange::Rerun);
            AnimColor::new(
                transparent_edge(previous.get(), *target),
                transparent_edge(*target, previous.get()),
            )
            .time(if *pressed || *reduced {
                0
            } else {
                super::tokens::HOVER_MS
            })
            .ease(Ease::Out)
            .function(Function::Cubic)
        },
    );
    let value = if pressed || reduced {
        target
    } else {
        animation.get().value()
    };
    current.set(value);
    value
}

/// A retargetable value: selection travels, press depth settles, layout stays fixed.
pub(super) fn travel(target: f32, duration: u64, curve: Function) -> f32 {
    let reduced = use_try_consume::<ReducedMotion>().is_none_or(|motion| *motion.0.read());
    let current = use_hook(|| Rc::new(Cell::new(target)));
    let previous = current.clone();
    let animation = use_animation_with_dependencies(
        &(target, reduced, duration, curve),
        move |config, (target, reduced, duration, curve)| {
            config.on_change(OnChange::Rerun);
            super::tokens::transition(previous.get(), *target, *reduced)
                .function(*curve)
                .time(if *reduced { 0 } else { *duration })
        },
    );
    let value = if reduced {
        target
    } else {
        animation.get().value()
    };
    current.set(value);
    value
}

/// A newly mounted floating surface settles into place without reflowing its contents.
#[derive(Clone, PartialEq)]
pub(super) struct Appear {
    pub content: Element,
    pub area: Area,
}
impl Component for Appear {
    fn render(&self) -> impl IntoElement {
        let reduced = use_try_consume::<ReducedMotion>().is_none_or(|motion| *motion.0.read());
        let progress = freya::animation::use_animation(move |config| {
            config.on_creation(freya::animation::OnCreation::Run);
            super::tokens::transition(0., 1., reduced)
        });
        let amount = if reduced { 1. } else { progress.get().value() };
        rect()
            .position(
                Position::new_global()
                    .left(self.area.min_x())
                    .top(self.area.min_y()),
            )
            .width(Size::px(self.area.width()))
            .height(Size::px(self.area.height()))
            .layer(Layer::Overlay)
            .opacity(amount)
            .offset_y(-super::tokens::SPACE_XS * (1. - amount))
            .child(self.content.clone())
    }
}

// Transparent black must not darken a surface on its way to the hover colour.
fn transparent_edge(color: Color, other: Color) -> Color {
    if color.a() == 0 {
        other.with_a(0)
    } else {
        color
    }
}
