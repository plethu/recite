use super::wedge::{SIDE, Wedge, WedgePress};
use super::*;
use crate::presentation::material::Decor;
use recite_config::WriterConfig;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, Role, TreeId},
    core::NewWidget,
    kurbo::{Point, Shape},
    layout::AsUnit,
    properties::{Background, Padding, types::CrossAxisAlignment},
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, Label, SizedBox},
};

#[test]
fn the_error_wedge_is_a_24px_named_target_carrying_its_problem_as_the_description() {
    let wedge = NewWidget::new(Wedge::new(
        "Problem in Relay Desk".into(),
        "unknown block reference `nowhere`".into(),
        (Color::from_rgb8(137, 51, 48), Color::from_rgb8(104, 62, 98)),
    ));
    let mut harness = TestHarness::create_with_size(default_property_set(), wedge, (100, 100));
    let id = harness.root_id();
    harness.redraw();
    let node = harness.access_node(id).expect("wedge node");
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.label().as_deref(), Some("Problem in Relay Desk"));
    assert_eq!(
        node.description().as_deref(),
        Some("unknown block reference `nowhere`")
    );
    let size = harness.get_widget_with_id(id).ctx().border_box().size();
    assert!(size.width >= SIDE && size.height >= SIDE);
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: None,
    });
    assert_eq!(harness.pop_action::<WedgePress>(), Some((WedgePress, id)));
}

#[test]
fn crop_marks_are_detached_outward_ticks_that_double_at_200_and_keep_clear_of_the_room() {
    let block = Rect::new(32., 100., 895., 400.);
    for (scale, sheet, room, gap, arm, weight) in [
        (1., true, 24., 4., 8., 2.),
        (2., true, 24., 8., 12., 4.),
        (2., true, 32., 8., 16., 4.),
        (1., false, 16., 2., 6., 2.),
    ] {
        let ticks = Ticks::new(scale, room, sheet);
        assert_eq!((ticks.gap, ticks.arm, ticks.weight), (gap, arm, weight));
        assert!(ticks.gap + ticks.arm <= room - CLEAR);
        for tick in ticks.rects(block) {
            // Every tick lies outside the block on the line of one of its edges, off the corner.
            assert!(
                tick.intersect(block).area() == 0.,
                "{tick:?} inside the block"
            );
            let long = tick.width().max(tick.height());
            assert_eq!((long, tick.width().min(tick.height())), (arm, weight));
            let corners = [
                Point::new(block.x0, block.y0),
                Point::new(block.x1, block.y0),
                Point::new(block.x0, block.y1),
                Point::new(block.x1, block.y1),
            ];
            let nearest = corners
                .iter()
                .map(|corner| {
                    let dx = (tick.x0 - corner.x).abs().min((tick.x1 - corner.x).abs());
                    let dy = (tick.y0 - corner.y).abs().min((tick.y1 - corner.y).abs());
                    dx.max(dy)
                })
                .fold(f64::INFINITY, f64::min);
            assert_eq!(nearest, gap, "{tick:?} is {gap}px off its corner");
        }
    }
    // A card's marks clear its stock offset as well as the card.
    let card = Ticks::new(1., 32., true).clearing(4.);
    let rects = card.rects(block);
    assert_eq!(rects[2].x0, block.x1 + 4. + 4.);
    assert_eq!(rects[5].y0, block.y1 + 4. + 4.);
}

#[test]
fn the_galley_dot_sits_in_the_pad_clear_of_the_crop_marks_and_the_margin() {
    for (scale, pad, sheet) in [
        (1., 32_f64, true),
        (2., 32., true),
        (1., 16., false),
        (2., 16., false),
    ] {
        let ticks = Ticks::new(scale, pad.min(24.), sheet);
        // The beat: its heading row, then lines down to the insertion row.
        let block = Rect::new(pad, 120., pad + 863., 120. + 420. * scale);
        let rects = ticks.rects(block);
        let heading = Rect::new(block.x0, block.y0, block.x1, block.y0 + 32. * scale);
        let mut dots = vec![Dot::galley(scale, pad, 0., false).circle(heading, 0.)];
        // A draft line's first baseline lies below the heading and above the insertion row; a
        // nested row's dot still sits beside the beat's own edge.
        let mut baseline = 52. * scale;
        while baseline < block.height() - 40. * scale {
            for inset in [0., 24., 48.] {
                let row = Rect::new(block.x0 + inset, block.y0, block.x1, block.y1);
                dots.push(Dot::galley(scale, pad, inset, true).circle(row, baseline));
            }
            baseline += 7. * scale;
        }
        for dot in dots {
            let bounds = dot.bounding_box();
            assert!((DOT..=DOT * scale).contains(&bounds.width()));
            // In the pad: on the sheet, and left of the margin column, where the collapse
            // chevron, the speaker label and the line's id start.
            assert!(bounds.x0 >= CLEAR, "{bounds:?} off the sheet at {scale}x");
            assert!(
                bounds.x1 <= block.x0 - 2. * scale,
                "{bounds:?} on the margin"
            );
            for tick in rects {
                assert!(
                    bounds.intersect(tick).area() == 0.,
                    "dot {bounds:?} meets tick {tick:?} at {scale}x"
                );
            }
        }
    }
}

/// `marks` around a 100×60 box in a 200×140 window on the sheet, rendered.
fn rendered(palette: &Palette<Color>, marks: Marks) -> image::RgbaImage {
    let decor = NewWidget::new(Decor::new(
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
    let palette = super::super::palette(&config);
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
    let palette = super::super::palette(&WriterConfig::default());
    let marks = Marks::crop(&palette, true, Ticks::new(1., 32., true)).unsaved(
        &palette,
        true,
        Dot::galley(1., 32., 0., true),
    );
    let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Relay Desk"))));
    let id = button.id();
    let decor = NewWidget::new(Decor::new(Decoration::Marks(marks), button));
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
