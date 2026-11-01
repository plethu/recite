use super::*;
use crate::keyed::keyed;
use crate::presentation::text_button;
use crate::tests::support::{self, NoProxy};
use recite_writer_session::map::synopsis;
use std::sync::Arc;
use xilem::core::DynMessage;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, TreeId},
    core::WidgetId,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{Button, Passthrough},
};

fn collection(
    graph: &Topology,
    indices: &[usize],
) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    let children = indices
        .iter()
        .map(|index| {
            let prose = synopsis::synopsis(&graph.blocks[*index].entries).prose;
            let response = prose.clone();
            (
                key(graph, *index),
                text_button(1., prose, move |writer: &mut Writer| {
                    writer.status = response.clone()
                }),
            )
        })
        .collect();
    keyed((), cards(children))
}
fn button(harness: &TestHarness<Passthrough>, text: &str) -> Option<WidgetId> {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if widget.downcast::<Button>().is_some()
            && support::native_label(harness, widget.id()).as_deref() == Some(text)
        {
            return Some(widget.id());
        }
        pending.extend(widget.children());
    }
    None
}
#[test]
fn duplicate_rows_reconcile_independently_and_retired_messages_cannot_reach_remounted_rows() {
    let directory = support::project(":: same default\n> @ffee6p8\n  First twin.\n:: same\n> @rthh7ab\n  Second twin.\n:: neighbour\n> @bqyvpqq\n  Neighbour.\n").unwrap();
    let mut writer = support::open(&directory).unwrap();
    let graph = Topology::new(
        writer.session.model().document().script_snapshot().unwrap(),
        &Default::default(),
    );
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = collection(&graph, &[0, 1, 2]);
    let (pod, mut local) = view.build(&mut ctx, &mut writer);
    let mut harness =
        TestHarness::create_with_size(default_property_set(), pod.new_widget, (800, 200));
    let first = button(&harness, "First twin.").unwrap();
    let second = button(&harness, "Second twin.").unwrap();
    let neighbour = button(&harness, "Neighbour.").unwrap();
    assert_ne!(first, second);
    let next = collection(&graph, &[0, 1, 2]);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut writer));
    view = next;
    harness.redraw();
    assert_eq!(button(&harness, "First twin."), Some(first));
    assert_eq!(button(&harness, "Second twin."), Some(second));
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: first.to_raw().into(),
        data: None,
    });
    let (action, source) = harness.pop_action_erased().unwrap();
    let old_path = ctx.get_id_path(source).unwrap().clone();
    let next = collection(&graph, &[1, 2]);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut writer));
    view = next;
    harness.redraw();
    assert!(button(&harness, "First twin.").is_none());
    assert_eq!(button(&harness, "Second twin."), Some(second));
    assert_eq!(button(&harness, "Neighbour."), Some(neighbour));
    let next = collection(&graph, &[0, 1, 2]);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut writer));
    view = next;
    harness.redraw();
    assert_ne!(button(&harness, "First twin.").unwrap(), first);
    assert_eq!(button(&harness, "Second twin."), Some(second));
    let before = writer.status.clone();
    let mut message = MessageCtx::new(
        std::mem::take(ctx.environment()),
        old_path,
        DynMessage(action),
    );
    let result =
        harness.edit_root_widget(|root| view.message(&mut local, &mut message, root, &mut writer));
    assert!(matches!(result, MessageResult::Stale));
    *ctx.environment() = message.finish().0;
    assert_eq!(writer.status, before);
    let mut blocks = graph.blocks.to_vec();
    blocks[0].id = "repaired".into();
    let mut inserted = blocks[2].clone();
    inserted.id = "inserted".into();
    blocks.insert(0, inserted);
    let repaired = Topology::new(blocks.into(), &Default::default());
    let next = collection(&repaired, &[1, 2, 3]);
    harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut writer));
    view = next;
    harness.redraw();
    assert_eq!(button(&harness, "Neighbour."), Some(neighbour));
    assert_eq!(button(&harness, "Second twin."), Some(second));
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: second.to_raw().into(),
        data: None,
    });
    let (action, source) = harness.pop_action_erased().unwrap();
    let mut message = MessageCtx::new(
        std::mem::take(ctx.environment()),
        ctx.get_id_path(source).unwrap().clone(),
        DynMessage(action),
    );
    let result =
        harness.edit_root_widget(|root| view.message(&mut local, &mut message, root, &mut writer));
    assert!(!matches!(result, MessageResult::Stale));
    *ctx.environment() = message.finish().0;
    assert_eq!(writer.status, "Second twin.");
    harness.edit_root_widget(|root| view.teardown(&mut local, &mut ctx, root));
}
