use super::support::NoProxy;
use crate::keyed::keyed;
use masonry::testing::TestHarness;
use std::sync::Arc;
use xilem::core::{DynMessage, MessageCtx, MessageResult, View as _, ViewPathTracker};
use xilem::{ViewCtx, WidgetView};

fn field(
    state: &(u64, String),
) -> impl WidgetView<(u64, String), Widget = masonry::widgets::Passthrough> + use<> {
    keyed(
        state.0,
        xilem::view::text_input(state.1.clone(), |state: &mut (u64, String), text| {
            state.1 = text
        }),
    )
}

#[test]
fn identity_changes_replace_native_fields_and_reject_messages_from_retired_fields() {
    let mut app = (0, "Equal text".to_owned());
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = field(&app);
    let (pod, mut local) = view.build(&mut ctx, &mut app);
    let mut harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        pod.new_widget,
        (400, 100),
    );
    let find = |harness: &TestHarness<masonry::widgets::Passthrough>, ctx: &ViewCtx| {
        let mut pending = vec![harness.root_widget().as_dyn()];
        while let Some(widget) = pending.pop() {
            if let Some(path) = ctx.get_id_path(widget.id()) {
                return Some((widget.id(), path.clone()));
            }
            pending.extend(widget.children());
        }
        None
    };
    let (old_id, old_path) = find(&harness, &ctx).unwrap();
    let next = field(&app);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut app));
    view = next;
    assert_eq!(
        find(&harness, &ctx).unwrap().0,
        old_id,
        "ordinary updates retain native editing state"
    );
    app.0 = 1;
    let next = field(&app);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut app));
    view = next;
    let (new_id, new_path) = find(&harness, &ctx).unwrap();
    assert_ne!(
        old_id, new_id,
        "equal text cannot make two document fields share native history"
    );
    for (path, expected_stale) in [(old_path, true), (new_path, false)] {
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            path,
            DynMessage(Box::new(masonry::widgets::TextAction::Changed(
                "New draft".into(),
            ))),
        );
        let result =
            harness.edit_root_widget(|root| view.message(&mut local, &mut message, root, &mut app));
        assert_eq!(matches!(result, MessageResult::Stale), expected_stale);
        *ctx.environment() = message.finish().0;
        assert_eq!(
            app.1,
            if expected_stale {
                "Equal text"
            } else {
                "New draft"
            }
        );
    }
    harness.edit_root_widget(|root| view.teardown(&mut local, &mut ctx, root));
}
