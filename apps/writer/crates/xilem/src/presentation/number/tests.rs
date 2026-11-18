use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionData, ActionRequest, Role, TreeId},
    core::{NewWidget, PointerButton, TextEvent, keyboard::Key, keyboard::NamedKey},
    kurbo::Point,
    testing::TestHarness,
    theme::default_property_set,
    widgets::Label,
};

fn field(value: f64) -> NewWidget<NumberField> {
    NewWidget::new(NumberField::new(
        "Reading text size".into(),
        format!("{value} px").into(),
        Range {
            value,
            min: 12.,
            max: 32.,
            step: 1.,
        },
        (Color::BLACK, 24.),
        NewWidget::new(Label::new(format!("{value} px")).accessibility_hidden(true)),
    ))
}

fn key(harness: &mut TestHarness<NumberField>, named: NamedKey) {
    harness.process_text_event(TextEvent::key_down(Key::Named(named)));
    harness.process_text_event(TextEvent::key_up(Key::Named(named)));
}

#[test]
fn a_number_field_is_one_named_spin_button_stepped_by_keys_pointer_and_assistive_actions() {
    let mut harness = TestHarness::create_with_size(default_property_set(), field(20.), (200, 60));
    let id = harness.root_id();
    harness.redraw();
    let node = harness.access_node(id).expect("spin button node");
    assert_eq!(node.role(), Role::SpinButton);
    assert_eq!(node.label().as_deref(), Some("Reading text size"));
    assert_eq!(node.value().as_deref(), Some("20 px"));
    assert_eq!(node.numeric_value(), Some(20.));
    assert!(
        node.data().supports_action(Action::Increment)
            && node.data().supports_action(Action::Decrement)
    );
    harness.focus_on(Some(id));
    for (named, expected) in [
        (NamedKey::ArrowUp, Spin::Step { up: true }),
        (NamedKey::ArrowRight, Spin::Step { up: true }),
        (NamedKey::ArrowDown, Spin::Step { up: false }),
        (NamedKey::Home, Spin::To(12.)),
        (NamedKey::End, Spin::To(32.)),
    ] {
        key(&mut harness, named);
        assert_eq!(harness.pop_action::<Spin>(), Some((expected, id)));
    }
    harness.process_access_event(ActionRequest {
        action: Action::Decrement,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: None,
    });
    assert_eq!(
        harness.pop_action::<Spin>(),
        Some((Spin::Step { up: false }, id))
    );
    // Assistive technology may set a value; it is kept within the range.
    harness.process_access_event(ActionRequest {
        action: Action::SetValue,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: Some(ActionData::NumericValue(40.)),
    });
    assert_eq!(harness.pop_action::<Spin>(), Some((Spin::To(32.), id)));
    // The minus face is the field's leading 24px; the plus face its trailing 24px.
    let edge = harness.get_widget_with_id(id).ctx().window_transform() * Point::ORIGIN;
    let width = harness.get_widget_with_id(id).ctx().border_box().width();
    harness.mouse_move((edge.x + 6., edge.y + 10.));
    harness.mouse_button_press(Some(PointerButton::Primary));
    harness.mouse_button_release(Some(PointerButton::Primary));
    assert_eq!(
        harness.pop_action::<Spin>(),
        Some((Spin::Step { up: false }, id))
    );
    harness.mouse_move((edge.x + width - 6., edge.y + 10.));
    harness.mouse_button_press(Some(PointerButton::Primary));
    harness.mouse_button_release(Some(PointerButton::Primary));
    assert_eq!(
        harness.pop_action::<Spin>(),
        Some((Spin::Step { up: true }, id))
    );
}

#[test]
fn a_number_field_at_a_bound_offers_and_takes_no_step_past_it() {
    let mut harness = TestHarness::create_with_size(default_property_set(), field(32.), (200, 60));
    let id = harness.root_id();
    harness.redraw();
    let node = harness.access_node(id).expect("spin button node");
    assert!(!node.data().supports_action(Action::Increment));
    assert!(node.data().supports_action(Action::Decrement));
    harness.focus_on(Some(id));
    key(&mut harness, NamedKey::ArrowUp);
    key(&mut harness, NamedKey::End);
    assert_eq!(harness.pop_action::<Spin>(), None);
    key(&mut harness, NamedKey::ArrowDown);
    assert_eq!(
        harness.pop_action::<Spin>(),
        Some((Spin::Step { up: false }, id))
    );
}
