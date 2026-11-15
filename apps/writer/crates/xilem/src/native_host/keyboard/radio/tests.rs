use super::super::tab::past_panes;
use super::*;
use crate::presentation::GroupStop;
use xilem::masonry::{
    core::NewWidget,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Flex, RadioButtonSelected, RadioGroup},
};

fn choice(checked: bool, text: &str) -> NewWidget<RadioButton> {
    NewWidget::new(RadioButton::new(checked, text)).with_props(GroupStop(checked))
}

#[test]
fn a_radio_group_is_one_tab_stop_and_arrows_choose_within_it() {
    let before = NewWidget::new(Button::with_text("Before"));
    let (script, map, split) = (
        choice(false, "Script"),
        choice(true, "Map"),
        choice(false, "Split"),
    );
    let after = NewWidget::new(Button::with_text("After"));
    let ids = [before.id(), script.id(), map.id(), split.id(), after.id()];
    let [before_id, script_id, map_id, split_id, after_id] = ids;
    let group = Flex::row()
        .with_fixed(script)
        .with_fixed(map)
        .with_fixed(split);
    let root = Flex::column()
        .with_fixed(before)
        .with_fixed(NewWidget::new(RadioGroup::new(NewWidget::new(group))))
        .with_fixed(after);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(root), (400, 200));
    // Tab lands on the checked choice and leaves the group on the next press.
    harness.focus_on(Some(before_id));
    harness.press_tab_key(false);
    past_panes(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(map_id));
    harness.press_tab_key(false);
    past_panes(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(after_id));
    harness.press_tab_key(true);
    past_panes(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(map_id));
    harness.press_tab_key(true);
    past_panes(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(before_id));
    // Arrows move focus and choose, wrapping at either end.
    step(harness.render_root(), map_id, false);
    harness.animate_ms(0);
    assert_eq!(harness.focused_widget_id(), Some(split_id));
    assert_eq!(
        harness.pop_action::<RadioButtonSelected>(),
        Some((RadioButtonSelected, split_id))
    );
    step(harness.render_root(), split_id, false);
    assert_eq!(harness.focused_widget_id(), Some(script_id));
    step(harness.render_root(), script_id, true);
    assert_eq!(harness.focused_widget_id(), Some(split_id));
}
