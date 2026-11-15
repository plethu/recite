use super::super::tab::to_tab_stop;
use super::*;
use crate::presentation::GroupStop;
use crate::presentation::group::Group;
use xilem::masonry::{
    core::NewWidget,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, ButtonAccessState, ButtonPress, Flex, RadioButtonSelected, RadioGroup},
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
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(map_id));
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(after_id));
    harness.press_tab_key(true);
    to_tab_stop(harness.render_root(), true);
    assert_eq!(harness.focused_widget_id(), Some(map_id));
    harness.press_tab_key(true);
    to_tab_stop(harness.render_root(), true);
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

fn tab(selected: bool, text: &str) -> NewWidget<Button> {
    NewWidget::new(
        Button::with_text(text)
            .with_role(Role::Tab)
            .with_accessible_label(text)
            .with_access_state(ButtonAccessState {
                selected: Some(selected),
                ..ButtonAccessState::default()
            })
            .with_focusable(selected),
    )
}

#[test]
fn a_tab_list_is_one_tab_stop_and_left_and_right_move_and_select() {
    let before = NewWidget::new(Button::with_text("Before"));
    let (write, localise) = (tab(true, "Write"), tab(false, "Localise"));
    let after = NewWidget::new(Button::with_text("After"));
    let [before_id, write_id, localise_id, after_id] =
        [before.id(), write.id(), localise.id(), after.id()];
    let list = Group::new(
        "Mode".into(),
        Role::TabList,
        NewWidget::new(Flex::row().with_fixed(write).with_fixed(localise)),
    );
    let root = Flex::column()
        .with_fixed(before)
        .with_fixed(NewWidget::new(list))
        .with_fixed(after);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), NewWidget::new(root), (400, 200));
    // Only the selected tab is a stop.
    harness.focus_on(Some(before_id));
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(write_id));
    harness.press_tab_key(false);
    to_tab_stop(harness.render_root(), false);
    assert_eq!(harness.focused_widget_id(), Some(after_id));
    // Right reaches the unselected tab, focuses and activates it, and wraps back.
    let mut arrows = Arrows::default();
    harness.focus_on(Some(write_id));
    assert!(arrows.request(
        harness.render_root(),
        &keyboard::Key::Named(NamedKey::ArrowRight)
    ));
    arrows.synchronize(harness.render_root());
    harness.animate_ms(0);
    assert_eq!(harness.focused_widget_id(), Some(localise_id));
    assert_eq!(
        harness.pop_action::<ButtonPress>().map(|(_, id)| id),
        Some(localise_id)
    );
    // Down belongs to the content below a tab row, not to the tabs.
    assert!(!arrows.request(
        harness.render_root(),
        &keyboard::Key::Named(NamedKey::ArrowDown)
    ));
    step(harness.render_root(), localise_id, false);
    assert_eq!(harness.focused_widget_id(), Some(write_id));
}
