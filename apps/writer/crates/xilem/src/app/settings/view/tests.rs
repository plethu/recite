use super::*;
use crate::tests::support::NoProxy;
use std::sync::Arc;
use xilem::{ViewCtx, core::View, masonry::testing::TestHarness};

#[test]
fn personal_controls_expose_their_native_checkbox_names() {
    let mut app = App::empty();
    let view = settings(&app);
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let (pod, _) = view.build(&mut ctx, &mut app);
    let harness = TestHarness::create_with_size(
        app.theme.initial_properties(),
        xilem::masonry::core::NewWidget::new(xilem::masonry::widgets::SizedBox::new(
            pod.new_widget,
        )),
        (1200, 650),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut labels = Vec::new();
    while let Some(node) = pending.pop() {
        if let Some(label) = node.label().or_else(|| node.value()) {
            labels.push(label);
        }
        pending.extend(node.children());
    }
    assert!(
        labels.iter().any(|label| label == "Confirm before closing"),
        "{labels:?}"
    );
}
