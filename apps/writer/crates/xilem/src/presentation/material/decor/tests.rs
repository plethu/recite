use super::*;
use xilem::masonry::{
    core::NewWidget,
    layout::AsUnit,
    properties::{Background, Padding},
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, Label},
};

#[test]
fn a_focus_ring_sits_inside_the_fill_only_while_focus_is_within() {
    let fill = Color::from_rgb8(104, 62, 98);
    let ring = Color::from_rgb8(255, 251, 245);
    let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Create project"))))
        .with_props(Background::Color(fill));
    let id = button.id();
    let decor = NewWidget::new(widget::Decor::new(Decoration::Ring(Some(ring)), button));
    let mut harness = TestHarness::create_with_size(
        default_property_set(),
        NewWidget::new(Flex::column().with_fixed(decor)),
        (300, 100),
    );
    let widget = harness.get_widget_with_id(id);
    let bounds = widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box());
    let middle = (bounds.y0 + bounds.height() / 2.) as u32;
    let x = |offset: f64| (bounds.x0 + offset) as u32;
    let pixel = |image: &image::RgbaImage, at: u32| image.get_pixel(at, middle).0;
    let rgba = |color: Color| color.to_rgba8().to_u8_array();
    let before = harness.render();
    assert_eq!(pixel(&before, x(3.)), rgba(fill));
    harness.focus_on(Some(id));
    let after = harness.render();
    // Fill, ring, fill: the ring never touches the surface around the control.
    assert_eq!(pixel(&after, x(3.)), rgba(ring));
    assert_eq!(pixel(&after, x(5.5)), rgba(fill));
    harness.focus_on(None);
    assert_eq!(pixel(&harness.render(), x(3.)), rgba(fill));
}

#[test]
fn a_focus_outline_sits_outside_the_field_and_takes_no_room() {
    let ink = Color::from_rgb8(104, 62, 98);
    let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Edit line"))));
    let id = button.id();
    let decor = NewWidget::new(widget::Decor::new(
        Decoration::Outline(Some(ink), 4.),
        button,
    ));
    let outer = decor.id();
    let mut harness = TestHarness::create_with_size(
        default_property_set(),
        NewWidget::new(Flex::column().with_fixed(decor)).with_props(Padding::all(20.px())),
        (300, 100),
    );
    let widget = harness.get_widget_with_id(id);
    let bounds = widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box());
    // The outline adds nothing to the field's box.
    let outer = harness.get_widget_with_id(outer).ctx().border_box().size();
    let inner = widget.ctx().border_box().size();
    assert!((outer.width - inner.width).abs() <= 1. && outer.height == inner.height);
    let middle = (bounds.y0 + bounds.height() / 2.) as u32;
    let rgba = |color: Color| color.to_rgba8().to_u8_array();
    let before = harness.render();
    // Its inner edge is 4px outside the field, and it is 2px wide.
    let at = (bounds.x0 - 5.) as u32;
    assert_ne!(before.get_pixel(at, middle).0, rgba(ink));
    harness.focus_on(Some(id));
    let after = harness.render();
    assert_eq!(after.get_pixel(at, middle).0, rgba(ink));
    assert_ne!(
        after.get_pixel((bounds.x0 - 2.) as u32, middle).0,
        rgba(ink)
    );
    harness.focus_on(None);
    assert_ne!(harness.render().get_pixel(at, middle).0, rgba(ink));
}
