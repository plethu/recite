//! Moving an ancestor re-emits its own accessibility node, not every unchanged descendant.
use masonry::{
    core::{NewWidget, WidgetId},
    kurbo::{Affine, Vec2},
    testing::TestHarness,
    widgets::{Flex, Label, Passthrough},
};

fn harness(rows: usize) -> (TestHarness<Passthrough>, WidgetId, Vec<WidgetId>) {
    let labels = (0..rows)
        .map(|row| NewWidget::new(Label::new(format!("Row {row}"))))
        .collect::<Vec<_>>();
    let ids = labels.iter().map(|label| label.id()).collect();
    let column = NewWidget::new(
        labels
            .into_iter()
            .fold(Flex::column(), |column, label| column.with_fixed(label)),
    );
    let column_id = column.id();
    let harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        NewWidget::new(Passthrough::new(column)),
        (400, 400),
    );
    (harness, column_id, ids)
}

#[test]
fn translating_an_ancestor_updates_only_that_node_and_descendant_bounds_follow() {
    let (mut harness, column, labels) = harness(64);
    let before = harness
        .access_node(labels[3])
        .and_then(|node| node.bounding_box())
        .unwrap();
    // Masonry re-emits the window root on every frame, including idle ones.
    let (_, idle) = harness.redraw();
    let mut expected = idle.nodes.iter().map(|(id, _)| id.0).collect::<Vec<_>>();
    expected.push(column.to_raw());
    expected.sort_unstable();
    let offset = Vec2::new(37., -120.);
    harness.edit_root_widget(|mut root| {
        Passthrough::child_mut(&mut root)
            .ctx
            .set_transform(Affine::translate(offset));
    });
    let (_, update) = harness.redraw();
    let mut updated = update.nodes.iter().map(|(id, _)| id.0).collect::<Vec<_>>();
    updated.sort_unstable();
    assert_eq!(updated, expected);
    let after = harness
        .access_node(labels[3])
        .and_then(|node| node.bounding_box())
        .unwrap();
    assert_eq!(
        (after.x0 - before.x0, after.y0 - before.y0),
        (offset.x, offset.y)
    );
    assert_eq!(after.size(), before.size());
}
