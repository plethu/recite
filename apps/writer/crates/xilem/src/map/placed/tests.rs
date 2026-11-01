use super::*;
use crate::tests::support::NoProxy;
use std::{cell::Cell, sync::Arc};
use xilem::masonry::accesskit;
use xilem::masonry::kurbo::{Point, Vec2};
use xilem::masonry::theme;
use xilem::masonry::widgets;
use xilem::{
    masonry::testing::TestHarness,
    view::{button, label},
};
type State = (bool, Cell<usize>);

fn control(affine: Affine) -> impl WidgetView<State, Widget = Passthrough> {
    crate::keyed::keyed(
        (),
        placed(
            affine,
            button(label("Select beat"), |value: &mut State| value.0 = true),
        ),
    )
}
#[test]
fn placement_preserves_focus_and_child_identity_with_matching_pointer_and_accessibility_bounds() {
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = control(Affine::translate((60., 30.)));
    let mut value = (false, Cell::new(0));
    let (pod, mut local) = view.build(&mut ctx, &mut value);
    let mut harness =
        TestHarness::create_with_size(theme::default_property_set(), pod.new_widget, (400, 200));
    let child = harness.root_widget().children()[0].children()[0].id();
    harness.focus_on(Some(child));
    let next = control(Affine::translate((100., 50.)));
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut value));
    assert_eq!(
        harness.root_widget().children()[0].children()[0].id(),
        child
    );
    assert_eq!(harness.focused_widget_id(), Some(child));
    let button = harness.get_widget_with_id(child);
    let bounds = button
        .ctx()
        .window_transform()
        .transform_rect_bbox(button.ctx().border_box());
    assert!(bounds.x0 >= 100.);
    harness.redraw();
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut found = false;
    while let Some(node) = pending.pop() {
        if node.role() == accesskit::Role::Button {
            assert!(
                node.bounding_box().unwrap().x0 >= 100.,
                "{:?}",
                node.bounding_box()
            );
            found = true;
        }
        pending.extend(node.children());
    }
    assert!(found);
    harness.mouse_click_on(child, None);
    assert!(harness.pop_action::<widgets::ButtonPress>().is_some());
}

fn nested(outer: Affine, inner: Affine) -> impl WidgetView<State, Widget = Passthrough> {
    crate::keyed::keyed(
        (),
        placed(
            outer,
            placed(
                inner,
                button(label("Nested card"), |value: &mut State| value.0 = true),
            ),
        ),
    )
}
#[test]
fn parent_placement_preserves_an_independent_child_transform() {
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut state = (false, Cell::new(0));
    let view = nested(Affine::translate((60., 30.)), Affine::scale(0.5));
    let (pod, mut local) = view.build(&mut ctx, &mut state);
    let mut harness =
        TestHarness::create_with_size(theme::default_property_set(), pod.new_widget, (400, 200));
    let child = harness.root_widget().children()[0].children()[0].children()[0].id();
    let before = harness.get_widget_with_id(child).ctx().window_transform();
    let next = nested(Affine::translate((100., 50.)), Affine::scale(0.5));
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut state));
    let actual = harness.get_widget_with_id(child).ctx().window_transform();
    assert_eq!(
        (actual * Point::ORIGIN) - (before * Point::ORIGIN),
        Vec2::new(40., 20.)
    );
    assert_eq!(
        (actual * Point::new(20., 10.)) - (actual * Point::ORIGIN),
        Vec2::new(10., 5.)
    );
    let last = nested(Affine::translate((100., 50.)), Affine::scale(0.75));
    harness.edit_root_widget(|root| last.rebuild(&next, &mut local, &mut ctx, root, &mut state));
    let actual = harness.get_widget_with_id(child).ctx().window_transform();
    assert_eq!(
        (actual * Point::new(20., 10.)) - (actual * Point::ORIGIN),
        Vec2::new(15., 7.5)
    );
}
