use super::*;
use xilem::masonry::{
    core::NewWidget,
    properties::Background,
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
