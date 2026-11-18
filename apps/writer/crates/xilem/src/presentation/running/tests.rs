use super::*;
use xilem::masonry::{
    core::NewWidget,
    layout::UnitPoint,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{ChildAlignment, Label, SizedBox, ZStack},
};

#[test]
fn a_running_head_takes_no_space_and_reads_nothing_until_shown() {
    let label = NewWidget::new(Label::new("Relay Hub › Relay Desk"));
    let label_id = label.id();
    let head = NewWidget::new(RunningHead::new(label));
    let mut harness = TestHarness::create_with_size(default_property_set(), head, (400, 100));
    let id = harness.root_id();
    assert!(harness.get_widget_with_id(label_id).ctx().is_stashed());
    harness.edit_root_widget(|mut head| RunningHead::set_shown(&mut head, true));
    harness.redraw();
    assert!(!harness.get_widget_with_id(label_id).ctx().is_stashed());
    assert!(harness.get_widget_with_id(id).ctx().border_box().height() > 0.);
    harness.edit_root_widget(|mut head| RunningHead::set_shown(&mut head, false));
    assert!(harness.get_widget_with_id(label_id).ctx().is_stashed());
}

#[test]
fn a_shown_running_head_keeps_its_own_height_over_a_tall_sheet() {
    let label = NewWidget::new(Label::new("Relay Hub › Relay Desk"));
    let head = NewWidget::new(RunningHead::new(label));
    let head_id = head.id();
    let stack = ZStack::new()
        .with_alignment(UnitPoint::TOP)
        .with(
            NewWidget::new(SizedBox::empty().size(400.px(), 600.px())),
            ChildAlignment::ParentAligned,
        )
        .with(head, ChildAlignment::ParentAligned);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(stack), (400, 600));
    harness.edit_widget_with_id(head_id, |mut head| {
        RunningHead::set_shown(&mut head.downcast::<RunningHead>(), true);
    });
    harness.redraw();
    // A head as tall as the sheet would paint over the page beneath it.
    let height = harness
        .get_widget_with_id(head_id)
        .ctx()
        .border_box()
        .height();
    assert!(height > 0. && height < 60., "{height}");
}
