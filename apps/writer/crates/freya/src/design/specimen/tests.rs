use freya::prelude::*;
use freya_testing::prelude::*;

#[test]
fn accessibility_click_updates_the_specimen_live_status() {
    let (mut test, _) = TestingRunner::new(super::app, (1200., 800.).into(), |_| {}, 1.);
    let status_id = test
        .find(|_, element| {
            let data = element.accessibility();
            (data.builder.role() == AccessibilityRole::Status)
                .then_some(data.a11y_id)
                .flatten()
        })
        .unwrap();
    let id = test
        .find(|_, element| {
            let data = element.accessibility();
            (data.builder.label() == Some("Try scene"))
                .then_some(data.a11y_id)
                .flatten()
        })
        .unwrap();
    assert!(
        test.send_accessibility_action(AccessibilityActionEventData {
            action: AccessibilityAction::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: None,
        })
    );
    let update = test.sync_and_update();
    assert!(
        update.nodes.iter().any(|(id, node)| {
            *id == status_id
                && node.role() == AccessibilityRole::Status
                && node.label() == Some("Primary action activated.")
                && node.live() == Some(accesskit::Live::Polite)
        }),
        "changed status must be published to the native adapter"
    );
    assert!(
        test.find(|_, element| {
            let data = element.accessibility();
            (data.builder.role() == AccessibilityRole::Status
                && data.builder.label() == Some("Primary action activated.")
                && data.builder.live() == Some(accesskit::Live::Polite))
            .then_some(())
        })
        .is_some()
    );
}
