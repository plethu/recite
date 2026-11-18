use super::*;
use xilem::masonry::{
    core::NewWidget,
    layout::AsUnit,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, Portal, SizedBox},
};

#[test]
fn panes_are_named_and_tab_reaches_their_controls_without_stopping_on_them() {
    let before = NewWidget::new(Button::with_text("Before"));
    let inside = NewWidget::new(Button::with_text("Inside"));
    let after = NewWidget::new(Button::with_text("After"));
    let (before_id, inside_id, after_id) = (before.id(), inside.id(), after.id());
    let content = Flex::column()
        .with_fixed(inside)
        .with_fixed(NewWidget::new(SizedBox::empty().height(600.px())));
    // As `presentation::pane` builds every Writer scroll area.
    let pane = NewWidget::new(
        Portal::new(NewWidget::new(content))
            .accessible_label("Script")
            .focusable(false),
    );
    let pane_id = pane.id();
    let root = Flex::column()
        .with_fixed(before)
        .with_fixed(NewWidget::new(SizedBox::new(pane).height(120.px())))
        .with_fixed(after);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(root), (300, 300));
    let node = harness.get_widget_with_id(pane_id);
    assert!(!node.ctx().accepts_focus());
    assert_eq!(node.accessibility_role(), Role::ScrollView);
    harness.focus_on(Some(before_id));
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(inside_id));
    // The pane's scrollbar stays a stop, so keyboard scrolling remains available.
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    let scrollbar = harness.focused_widget().unwrap();
    assert_eq!(scrollbar.accessibility_role(), Role::ScrollBar);
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(after_id));
    harness.focus_on(Some(inside_id));
    harness.press_tab_key(true);
    to_tab_stop(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(before_id));
    // A control that is a stop keeps its focus.
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(before_id));
}

#[test]
fn tab_cycles_within_a_mounted_modal_surface() {
    let behind = NewWidget::new(Button::with_text("Behind"));
    let first = NewWidget::new(Button::with_text("Keep editing"));
    let last = NewWidget::new(Button::with_text("Close"));
    let (behind_id, first_id, last_id) = (behind.id(), first.id(), last.id());
    let surface = NewWidget::new(SizedBox::new(NewWidget::new(
        Flex::column().with_fixed(first).with_fixed(last),
    )))
    .with_props(ModalSurface(true));
    let root = Flex::column().with_fixed(behind).with_fixed(surface);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(root), (300, 300));
    harness.focus_on(Some(last_id));
    // Masonry alone would leave the surface for the document behind it.
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(first_id));
    harness.press_tab_key(true);
    to_tab_stop(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(last_id));
    assert_ne!(harness.focused_widget_id(), Some(behind_id));
}
