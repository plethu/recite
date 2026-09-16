//! Exclusive choices with one tab stop and arrow-key selection.
use super::{Button, tokens as t};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct Options {
    pub name: &'static str,
    pub labels: [&'static str; 2],
    pub ids: [AccessibilityId; 2],
    pub selected: usize,
    pub change: EventHandler<usize>,
}

impl Component for Options {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let ids = self.ids;
        let selected = self.selected;
        let change = self.change.clone();
        let mut group = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::px(t::OPTION_GROUP_WIDTH))
            .border(Border::new().width(1.).fill(colors.rule))
            .spacing(t::SPACE_XS)
            .padding(t::SPACE_XS)
            .corner_radius(t::RADIUS)
            .background(colors.surface)
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt(self.name)
            .on_key_down(move |event: Event<KeyboardEventData>| {
                let next = match event.key {
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => 1 - selected,
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => 1 - selected,
                    Key::Named(NamedKey::Home) => 0,
                    Key::Named(NamedKey::End) => 1,
                    _ => return,
                };
                event.stop_propagation();
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
                    .child(*caption),
            );
        }
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(label().text(self.name)))
            .child(group)
    }
}
