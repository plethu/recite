use super::*;
use crate::tests::support::{self, NoProxy, open, project};
use xilem::masonry::accesskit;
use xilem::masonry::theme;
use xilem::{
    core::{DynMessage, MessageResult},
    masonry::testing::TestHarness,
};

#[test]
fn absent_present_replaced_and_removed_children_reject_retired_actions() {
    let first = project(recite_writer_model::FIXTURE).unwrap();
    let second =
        project(&recite_writer_model::FIXTURE.replace("Would you tell me", "New writer asks"))
            .unwrap();
    let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
    let mut ctx = ViewCtx::new(Arc::new(NoProxy), runtime);
    let mut app = App::empty();
    let mut view = writer_slot(&mut app);
    let (pod, mut state) = view.build(&mut ctx, &mut app);
    let mut harness =
        TestHarness::create_with_size(theme::default_property_set(), pod.new_widget, (900, 650));
    assert!(state.child.is_none());
    app.writer = Some(open(&first).unwrap());
    let next = writer_slot(&mut app);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut state, &mut ctx, root, &mut app));
    view = next;
    harness.redraw();
    assert!(state.child.is_some());
    let mut pending = vec![harness.root_widget().as_dyn()];
    let add = loop {
        let widget = pending.pop().expect("Add line action");
        if ctx.get_id_path(widget.id()).is_some()
            && support::native_label(&harness, widget.id()).as_deref() == Some("Add line")
        {
            break widget.id();
        }
        pending.extend(widget.children());
    };
    harness.process_access_event(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: add.to_raw().into(),
        data: None,
    });
    let (action, source) = harness.pop_action_erased().unwrap();
    let old_path = ctx.get_id_path(source).unwrap().clone();
    app.writer = Some(open(&second).unwrap());
    let before = app
        .writer
        .as_ref()
        .unwrap()
        .session
        .model()
        .document()
        .source()
        .to_owned();
    let next = writer_slot(&mut app);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut state, &mut ctx, root, &mut app));
    view = next;
    let mut message = MessageCtx::new(
        std::mem::take(ctx.environment()),
        old_path,
        DynMessage(action),
    );
    let result =
        harness.edit_root_widget(|root| view.message(&mut state, &mut message, root, &mut app));
    assert!(matches!(result, MessageResult::Stale));
    assert_eq!(
        app.writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        before
    );
    app.writer = None;
    let next = writer_slot(&mut app);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut state, &mut ctx, root, &mut app));
    assert!(state.child.is_none());
}
