use super::*;
use xilem::masonry::{
    core::NewWidget,
    layout::AsUnit,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, Portal, SizedBox},
};

#[test]
fn tab_passes_over_unnamed_scroll_panes_in_both_directions() {
    let before = NewWidget::new(Button::with_text("Before"));
    let inside = NewWidget::new(Button::with_text("Inside"));
    let after = NewWidget::new(Button::with_text("After"));
    let (before_id, inside_id, after_id) = (before.id(), inside.id(), after.id());
    let content = Flex::column()
        .with_fixed(inside)
        .with_fixed(NewWidget::new(SizedBox::empty().height(600.px())));
    let pane = NewWidget::new(Portal::new(NewWidget::new(content)));
    let pane_id = pane.id();
    let root = Flex::column()
        .with_fixed(before)
        .with_fixed(NewWidget::new(SizedBox::new(pane).height(120.px())))
        .with_fixed(after);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(root), (300, 300));
    assert!(harness.get_widget_with_id(pane_id).ctx().accepts_focus());
    harness.focus_on(Some(before_id));
    // Masonry alone stops on the pane, which has no name and draws no focus.
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(pane_id));
    past_panes(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(inside_id));
    // The pane's scrollbar stays a stop, so keyboard scrolling remains available.
    harness.press_tab_key(false);
    past_panes(harness.render_root(), false);
    let scrollbar = harness.focused_widget().unwrap();
    assert_eq!(scrollbar.accessibility_role(), Role::ScrollBar);
    harness.press_tab_key(false);
    past_panes(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(after_id));
    harness.focus_on(Some(inside_id));
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(pane_id));
    past_panes(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(before_id));
    // A control keeps its focus; only panes are passed over.
    past_panes(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(before_id));
}
