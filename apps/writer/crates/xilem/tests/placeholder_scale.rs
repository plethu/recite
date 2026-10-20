//! Placeholder glyphs follow both initial native defaults and live scaling.
use masonry::{
    core::{DefaultProperties, NewWidget, PropertySet},
    peniko::Color,
    properties::{Background, PlaceholderColor, PlaceholderTextSize},
    testing::TestHarness,
    widgets::TextInput,
};
use std::sync::Arc;

fn defaults(size: f32) -> DefaultProperties {
    let mut defaults = masonry::theme::default_property_set();
    defaults.insert::<TextInput, _>(PlaceholderTextSize { size });
    defaults
}
fn input(size: f32) -> TestHarness<TextInput> {
    TestHarness::create_with_size(
        defaults(size),
        NewWidget::new(TextInput::new("").with_placeholder("Search query")).with_props(
            PropertySet::new()
                .with(Background::Color(Color::WHITE))
                .with(PlaceholderColor::new(Color::BLACK)),
        ),
        (400, 100),
    )
}
fn glyph_ink(harness: &mut TestHarness<TextInput>) -> usize {
    harness
        .render()
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            *x > 5
                && *x < 395
                && *y > 5
                && *y < 95
                && pixel.0[0] < 150
                && pixel.0[1] < 150
                && pixel.0[2] < 150
        })
        .count()
}

#[test]
fn placeholder_scales_on_mount_and_on_default_property_changes() {
    let mut harness = input(14.);
    let small = glyph_ink(&mut harness);
    assert!(small > 0);
    harness.set_default_properties(Arc::new(defaults(28.)));
    harness.redraw();
    let large = glyph_ink(&mut harness);
    assert!(
        large > small * 2,
        "font scaling must increase rendered glyph area: {small} -> {large}"
    );
    assert_eq!(large, glyph_ink(&mut input(28.)));
    harness.set_default_properties(Arc::new(defaults(14.)));
    harness.redraw();
    assert_eq!(glyph_ink(&mut harness), small);
}
