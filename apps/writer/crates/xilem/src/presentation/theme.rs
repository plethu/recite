//! Toolkit property defaults retain authored colours through native control states.
#[cfg(test)]
mod tests;

use super::control_style::ControlStyle;
use super::*;
use xilem::masonry::theme;
use xilem::masonry::{
    core::{DefaultProperties, PropertySet, PropertyStack, Selector, Widget},
    layout::AsUnit,
    properties::{
        Background, BorderColor, BorderWidth, CaretColor, CheckmarkColor, CheckmarkSize,
        CheckmarkStrokeWidth, ContentColor, CornerRadius, Padding, PlaceholderColor,
        PlaceholderTextSize, SelectionColor,
    },
    widgets::{Button, Checkbox, Label, RadioButton, TextArea, TextInput},
};

pub(super) fn properties(p: &Palette<Color>, scale: f32) -> DefaultProperties {
    let mut properties = theme::default_property_set();
    text::<Label>(&mut properties, p);
    text::<TextArea<true>>(&mut properties, p);
    text::<TextArea<false>>(&mut properties, p);
    control::<Button>(&mut properties, p, scale);
    control::<TextInput>(&mut properties, p, scale);
    control::<Checkbox>(&mut properties, p, scale);
    control::<RadioButton>(&mut properties, p, scale);
    properties.insert::<Checkbox, _>(CheckmarkColor { color: p.accent });
    properties.insert::<RadioButton, _>(CheckmarkColor { color: p.accent });
    indicators::<Checkbox>(&mut properties, scale);
    indicators::<RadioButton>(&mut properties, scale);
    properties.insert::<TextInput, _>(Background::Color(p.surface));
    properties.insert::<TextInput, _>(PlaceholderColor::new(p.placeholder));
    properties.insert::<TextInput, _>(PlaceholderTextSize { size: 14. * scale });
    properties.insert::<TextInput, _>(CaretColor { color: p.ink });
    properties.insert::<TextInput, _>(SelectionColor { color: p.selection });
    properties.insert::<TextArea<true>, _>(CaretColor { color: p.ink });
    properties.insert::<TextArea<true>, _>(SelectionColor { color: p.selection });
    properties.insert::<TextArea<false>, _>(SelectionColor { color: p.selection });
    properties
}
fn indicators<W: Widget>(properties: &mut DefaultProperties, scale: f32) {
    let scale = f64::from(scale);
    properties.insert::<W, _>(CheckmarkSize {
        side: (theme::BASIC_WIDGET_HEIGHT.get() * scale).px(),
        gap: (theme::WIDGET_CONTROL_COMPONENT_PADDING.get() * scale).px(),
    });
    properties.insert::<W, _>(CheckmarkStrokeWidth { width: scale });
    properties.insert::<W, _>(BorderWidth { width: scale.px() });
}
fn text<W: Widget>(properties: &mut DefaultProperties, p: &Palette<Color>) {
    properties.insert::<W, _>(ContentColor::new(p.ink));
    let mut stack = PropertyStack::new();
    stack.push_layer(
        Selector::new().with_disabled(true),
        ContentColor::new(p.muted),
    );
    properties.insert_stack::<W>(stack);
}
fn control<W: Widget>(properties: &mut DefaultProperties, p: &Palette<Color>, scale: f32) {
    let scale = f64::from(scale);
    properties.insert::<W, _>(Background::Color(p.inset));
    properties.insert::<W, _>(BorderColor { color: p.boundary });
    properties.insert::<W, _>(BorderWidth { width: 1.px() });
    properties.insert::<W, _>(CornerRadius { radius: 3.px() });
    properties.insert::<W, _>(Padding::from_vh((6. * scale).px(), (12. * scale).px()));
    let mut stack = PropertyStack::new();
    stack.push_layer(
        Selector::new().with_hovered(true),
        Background::Color(p.hover),
    );
    stack.push_layer(
        Selector::new().with_active(true),
        Background::Color(p.pressed),
    );
    stack.push_layer(
        Selector::new().with_focused(true),
        BorderColor { color: p.accent },
    );
    stack.push_layer(
        Selector::new().with_disabled(true),
        PropertySet::new()
            .with(Background::Color(p.inset))
            .with(BorderColor { color: p.rule }),
    );
    quiet_controls(&mut stack, p);
    properties.insert_stack::<W>(stack);
}

fn quiet_controls(stack: &mut PropertyStack, p: &Palette<Color>) {
    for style in [
        ControlStyle::Action,
        ControlStyle::Navigation,
        ControlStyle::Segment,
        ControlStyle::Manuscript,
    ] {
        let classes = [style.class()];
        stack.push_layer(
            Selector::classes(&classes),
            PropertySet::new()
                .with(Background::Color(Color::TRANSPARENT))
                .with(BorderColor {
                    color: Color::TRANSPARENT,
                }),
        );
        stack.push_layer(
            Selector::classes(&classes).with_hovered(true),
            PropertySet::new().with(Background::Color(p.hover)),
        );
        stack.push_layer(
            Selector::classes(&classes).with_active(true),
            Background::Color(p.pressed),
        );
        stack.push_layer(
            Selector::classes(&classes).with_focused(true),
            BorderColor { color: p.accent },
        );
        stack.push_layer(
            Selector::classes(&classes).with_disabled(true),
            PropertySet::new()
                .with(Background::Color(Color::TRANSPARENT))
                .with(BorderColor {
                    color: Color::TRANSPARENT,
                }),
        );
    }
}
