//! Toolkit property defaults retain authored colours through native control states.
use super::*;
use xilem::masonry::{
    core::{DefaultProperties, PropertySet, PropertyStack, Selector, Widget},
    layout::AsUnit,
    properties::{
        Background, BorderColor, BorderWidth, CaretColor, CheckmarkColor, ContentColor,
        CornerRadius, Padding, PlaceholderColor, PlaceholderTextSize, SelectionColor,
    },
    widgets::{Button, Checkbox, Label, RadioButton, TextArea, TextInput},
};

pub(super) fn properties(p: &Palette<Color>, scale: f32) -> DefaultProperties {
    let mut properties = xilem::masonry::theme::default_property_set();
    text::<Label>(&mut properties, p);
    text::<TextArea<true>>(&mut properties, p);
    text::<TextArea<false>>(&mut properties, p);
    control::<Button>(&mut properties, p, scale);
    control::<TextInput>(&mut properties, p, scale);
    control::<Checkbox>(&mut properties, p, scale);
    control::<RadioButton>(&mut properties, p, scale);
    properties.insert::<Checkbox, _>(CheckmarkColor { color: p.accent });
    properties.insert::<RadioButton, _>(CheckmarkColor { color: p.accent });
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
    properties.insert_stack::<W>(stack);
}
