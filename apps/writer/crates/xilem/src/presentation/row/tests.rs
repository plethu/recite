use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, Role, TreeId},
    core::{NewWidget, TextEvent, keyboard::Key, keyboard::NamedKey},
    testing::TestHarness,
    theme::default_property_set,
    widgets::Label,
};

#[test]
fn a_row_is_a_named_button_whose_selection_is_a_state_not_part_of_its_name() {
    let row = NewWidget::new(RowButton::new(
        true,
        NewWidget::new(Label::new("Relay Desk")),
    ));
    let mut harness = TestHarness::create_with_size(default_property_set(), row, (300, 60));
    let id = harness.root_id();
    harness.redraw();
    let node = harness.access_node(id).expect("row node");
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.label().as_deref(), Some("Relay Desk"));
    assert_eq!(node.is_selected(), Some(true));
    harness.edit_root_widget(|mut row| RowButton::set_selected(&mut row, false));
    harness.redraw();
    assert_eq!(harness.access_node(id).unwrap().is_selected(), Some(false));
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: None,
    });
    assert_eq!(harness.pop_action::<RowPress>(), Some((RowPress, id)));
    harness.focus_on(Some(id));
    harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Enter)));
    harness.process_text_event(TextEvent::key_up(Key::Named(NamedKey::Enter)));
    assert_eq!(harness.pop_action::<RowPress>(), Some((RowPress, id)));
}
