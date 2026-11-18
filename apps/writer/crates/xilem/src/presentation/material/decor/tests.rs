use super::*;
use crate::presentation::marks::{Dot, DotAt, Ticks};
use recite_config::WriterConfig;
use recite_writer_session::palette::Palette;
use xilem::masonry::{
    core::NewWidget,
    kurbo::Point,
    layout::AsUnit,
    properties::{Background, Padding, types::CrossAxisAlignment},
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, Label, SizedBox},
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

/// `marks` around a 100×60 box in a 200×140 window on the sheet, rendered.
fn rendered(palette: &Palette<Color>, marks: Marks) -> image::RgbaImage {
    let decor = NewWidget::new(widget::Decor::new(
        Decoration::Marks(marks),
        NewWidget::new(SizedBox::empty().size(100.px(), 60.px())),
    ));
    let mut harness = TestHarness::create_with_size(
        default_property_set(),
        NewWidget::new(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_fixed(decor),
        )
        .with_props(Padding::all(40.px()))
        .with_props(Background::Color(palette.surface)),
        (200, 140),
    );
    harness.render()
}

#[test]
fn without_hue_current_unsaved_and_plain_still_differ_by_shape() {
    let config = WriterConfig {
        monochrome: true,
        ..WriterConfig::default()
    };
    let palette = crate::presentation::palette(&config);
    let ticks = Ticks::new(1., 32., true);
    let dot = Dot::new(1., DotAt::At(-12., 30.));
    let current = Marks::crop(&palette, true, ticks);
    let unsaved = Marks::default().unsaved(&palette, true, dot);
    let plain = Marks::default();
    // The top-leading tick across, 4px left of the box at (40, 40), and the dot's centre.
    let tick = (40 - 4 - 4, 40);
    let disc = (40 - 12, 40 + 30);
    let luminance = |image: &image::RgbaImage, (x, y): (u32, u32)| {
        let [r, g, b, _] = image.get_pixel(x, y).0;
        0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)
    };
    let images = [current, unsaved, plain].map(|marks| rendered(&palette, marks));
    let paper = luminance(&images[2], tick);
    assert_eq!(paper, luminance(&images[2], disc));
    // Ink is the darkest grey, so each mark shows only where its shape is.
    assert!(luminance(&images[0], tick) < paper / 2.);
    assert_eq!(luminance(&images[0], disc), paper);
    assert!(luminance(&images[1], disc) < paper / 2.);
    assert_eq!(luminance(&images[1], tick), paper);
}

#[test]
fn marks_take_no_room_and_no_pointer() {
    let palette = crate::presentation::palette(&WriterConfig::default());
    let marks = Marks::crop(&palette, true, Ticks::new(1., 32., true)).unsaved(
        &palette,
        true,
        Dot::galley(1., 32., 0., true),
    );
    let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Relay Desk"))));
    let id = button.id();
    let decor = NewWidget::new(widget::Decor::new(Decoration::Marks(marks), button));
    let outer = decor.id();
    let harness = TestHarness::create_with_size(
        default_property_set(),
        NewWidget::new(Flex::column().with_fixed(decor)).with_props(Padding::all(40.px())),
        (300, 140),
    );
    let inner = harness.get_widget_with_id(id).ctx().border_box().size();
    let decorated = harness.get_widget_with_id(outer).ctx().border_box().size();
    assert!((inner.width - decorated.width).abs() <= 1. && inner.height == decorated.height);
    let origin = harness
        .get_widget_with_id(id)
        .ctx()
        .window_transform()
        .transform_rect_bbox(harness.get_widget_with_id(id).ctx().border_box());
    // Over a tick and over the dot, the pointer finds the page beneath, never the marks.
    for point in [
        Point::new(origin.x0 - 8., origin.y0 + 1.),
        Point::new(origin.x0 - 12., origin.y0 + 10.),
    ] {
        let hit = harness
            .root_widget()
            .as_dyn()
            .find_widget_under_pointer(point)
            .map(|widget| widget.id());
        assert!(
            hit != Some(id) && hit != Some(outer),
            "{point:?} hits the marks"
        );
    }
}
