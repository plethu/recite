use super::wedge::{SIDE, Wedge, WedgePress};
use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, Role, TreeId},
    core::NewWidget,
    kurbo::{Point, Shape},
    testing::TestHarness,
    theme::default_property_set,
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
