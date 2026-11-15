use super::*;
use xilem::masonry::{
    core::NewWidget, testing::TestHarness, theme::default_property_set, widgets::Label,
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
