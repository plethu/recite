//! Exclusive choices with a shared selection plate and arrow-key navigation.
use super::{Button, tokens as t};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct Options<const N: usize = 2> {
    pub name: String,
    pub labels: [String; N],
    pub ids: [AccessibilityId; N],
    pub selected: usize,
    pub vim: bool,
    pub change: EventHandler<usize>,
}

impl<const N: usize> Component for Options<N> {
    fn render(&self) -> impl IntoElement {
        let row = if t::ui_scale() > 1.25 {
            rect()
        } else {
            rect().horizontal()
        };
        row.width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .width(if t::ui_scale() > 1.25 {
                        Size::fill()
                    } else {
                        Size::flex(1.)
                    })
                    .child(label().text(self.name.clone())),
            )
            .child(Segments {
                name: self.name.clone(),
                labels: self.labels.clone(),
                ids: self.ids,
                selected: self.selected,
                vim: self.vim,
                change: self.change.clone(),
                width: if t::ui_scale() > 1.25 {
                    Size::fill()
                } else {
                    Size::px(t::OPTION_GROUP_WIDTH)
                },
            })
    }
}

/// Shared exclusive view and preference switch with arrow-key selection.
#[derive(Clone, PartialEq)]
pub(crate) struct Segments<const N: usize = 2> {
    pub name: String,
    pub labels: [String; N],
    pub ids: [AccessibilityId; N],
    pub selected: usize,
    pub vim: bool,
    pub change: EventHandler<usize>,
    pub width: Size,
}
impl<const N: usize> Component for Segments<N> {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let mut size = use_state(|| Size2D::new(0., 0.));
        let amount = super::motion::travel(
            self.selected as f32,
            t::SELECTION_MS,
            freya::animation::Function::Quart,
        );
        let segment_width = ((size.read().width - 2. * t::SPACE_XS) / N as f32).max(0.);
        let indicator = rect()
            .position(
                Position::new_absolute()
                    .left(segment_width * amount)
                    .top(0.),
            )
            .width(Size::px(segment_width))
            .height(Size::px((size.read().height - 2. * t::SPACE_XS).max(0.)))
            .corner_radius(t::RADIUS)
            .background(colors.selection)
            .border(Border::new().width(1.).fill(colors.rule));
        let ids = self.ids;
        let selected = self.selected;
        let vim = self.vim;
        let change = self.change.clone();
        let mut group = rect()
            .horizontal()
            .content(Content::Flex)
            .width(self.width.clone())
            .padding(t::SPACE_XS)
            .corner_radius(t::RADIUS)
            .background(colors.inset)
            .border(Border::new().width(1.).fill(colors.rule))
            .on_sized(move |event: Event<SizedEventData>| size.set_if_modified(event.area.size))
            .child(indicator)
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt(self.name.clone())
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if !event.modifiers.is_empty() {
                    return;
                }
                let next = if let Some(step) = super::keyboard::list_step(&event, vim) {
                    (selected as isize + step).rem_euclid(N as isize) as usize
                } else {
                    match event.key {
                        Key::Named(NamedKey::ArrowLeft) => (selected + N - 1) % N,
                        Key::Named(NamedKey::ArrowRight) => (selected + 1) % N,
                        Key::Named(NamedKey::Home) => 0,
                        Key::Named(NamedKey::End) => N - 1,
                        _ => return,
                    }
                };
                event.stop_propagation();
                event.prevent_default();
                change.call(next);
                ids[next].request_focus();
            });
        for (index, caption) in self.labels.iter().enumerate() {
            let change = self.change.clone();
            group = group.child(
                Button::new()
                    .flat()
                    .radio(index == selected)
                    .width(Size::flex(1.))
                    .a11y_id(ids[index])
                    .named(format!("{}: {caption}", self.name))
                    .on_press(move |_| change.call(index))
                    .child(caption.clone()),
            );
        }
        group
    }
}

#[cfg(test)]
mod tests;
