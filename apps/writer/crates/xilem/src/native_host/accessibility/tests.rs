use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, TreeId},
    testing::TestHarness,
    theme,
    widgets::{Button, Passthrough},
};

#[test]
fn retired_click_is_ignored_and_replacement_actions_work_before_another_redraw() {
    let mut harness = TestHarness::create(
        theme::default_property_set(),
        Passthrough::new(Button::with_text("Cancel").prepare()).prepare(),
    );
    let old = harness.root_widget().children()[0].id();
    assert!(target_exists(harness.root_widget().as_dyn(), old.into()));
    harness.edit_root_widget(|mut root| {
        Passthrough::set_child(&mut root, Button::with_text("Next").prepare())
    });
    let current = harness.root_widget().children()[0].id();
    assert_ne!(old, current);
    let stale = ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: old.into(),
        data: None,
    };
    if target_exists(harness.root_widget().as_dyn(), stale.target_node) {
        harness.process_access_event(stale);
    }
    assert!(harness.pop_action_erased().is_none());
    for action in [Action::Focus, Action::Click] {
        let request = ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: current.into(),
            data: None,
        };
        assert!(target_exists(
            harness.root_widget().as_dyn(),
            request.target_node
        ));
        harness.process_access_event(request);
    }
    assert_eq!(harness.focused_widget_id(), Some(current));
    assert_eq!(harness.pop_action_erased().unwrap().1, current);
}
