use super::*;
use accesskit::{
    ActionRequest, Node as NodeData, NodeId as LocalId, Rect, Tree as TreeData, TreeId,
};
use std::sync::Mutex;

struct Callback(Arc<Mutex<usize>>);
impl AdapterCallback for Callback {
    fn register_interfaces(&self, _: &Adapter, _: NodeId, _: InterfaceSet) {}
    fn unregister_interfaces(&self, _: &Adapter, _: NodeId, _: InterfaceSet) {}
    fn emit_event(&self, _: &Adapter, event: Event) {
        if matches!(
            event,
            Event::Object {
                event: ObjectEvent::PropertyChanged(crate::Property::Name(_)),
                ..
            }
        ) {
            *self.0.lock().unwrap() += 1;
        }
    }
}
struct Actions;
impl ActionHandler for Actions {
    fn do_action(&mut self, _: ActionRequest) {}
}

#[test]
fn same_bounds_text_edit_does_not_walk_the_thousand_control_region() {
    let mut root = NodeData::new(Role::Window);
    root.set_children(vec![LocalId(1)]);
    let mut viewport = NodeData::new(Role::ScrollView);
    viewport.set_clips_children();
    viewport.set_bounds(Rect::new(0., 0., 100., 100.));
    viewport.set_children((2..1002).map(LocalId).collect::<Vec<_>>());
    let mut nodes = vec![(LocalId(0), root), (LocalId(1), viewport)];
    for index in 2..1002 {
        let mut button = NodeData::new(Role::Button);
        button.set_bounds(Rect::new(
            0.,
            (index - 2) as f64 * 40.,
            100.,
            (index - 1) as f64 * 40.,
        ));
        nodes.push((LocalId(index), button));
    }
    let names = Arc::new(Mutex::new(0));
    let adapter = Adapter::new(
        &AppContext::new(None),
        Callback(Arc::clone(&names)),
        TreeUpdate {
            nodes: nodes.clone(),
            tree: Some(TreeData::new(LocalId(0))),
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        false,
        WindowBounds::default(),
        Actions,
    );
    let mut handler = AdapterChangeHandler::new(&adapter);
    let mut tree = adapter.context.tree.write().unwrap();
    let mut button = nodes[2].1.clone();
    button.set_label("Edited control");
    tree.update_and_process_changes(
        TreeUpdate {
            nodes: vec![(LocalId(2), button)],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        &mut handler,
    );
    assert!(handler.exposure.old_subtrees.is_empty());
    assert!(handler.exposure.new_subtrees.is_empty());
    assert_eq!(handler.exposure.updated.len(), 1);
    assert_eq!(
        *names.lock().unwrap(),
        1,
        "the label change still reaches the platform"
    );
}

#[test]
fn camera_transform_reconciles_a_control_region_with_bounded_filter_work() {
    const CONTROLS: u64 = 128;
    let mut root = NodeData::new(Role::Window);
    root.set_children(vec![LocalId(1)]);
    let mut viewport = NodeData::new(Role::ScrollView);
    viewport.set_clips_children();
    viewport.set_bounds(Rect::new(0., 0., 1000., 1000.));
    viewport.set_children(vec![LocalId(2)]);
    let mut camera = NodeData::new(Role::GenericContainer);
    camera.set_children((3..3 + CONTROLS).map(LocalId).collect::<Vec<_>>());
    let mut nodes = vec![
        (LocalId(0), root),
        (LocalId(1), viewport),
        (LocalId(2), camera.clone()),
    ];
    for index in 3..3 + CONTROLS {
        let mut button = NodeData::new(Role::Button);
        button.set_label(format!("Beat {index}"));
        button.set_bounds(Rect::new(0., 0., 100., 100.));
        nodes.push((LocalId(index), button));
    }
    let adapter = Adapter::new(
        &AppContext::new(None),
        Callback(Arc::new(Mutex::new(0))),
        TreeUpdate {
            nodes,
            tree: Some(TreeData::new(LocalId(0))),
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        false,
        WindowBounds::default(),
        Actions,
    );
    let mut handler = AdapterChangeHandler::new(&adapter);
    let mut tree = adapter.context.tree.write().unwrap();
    camera.set_transform(accesskit::Affine::translate((20., 10.)));
    crate::filters::take_filter_calls();
    tree.update_and_process_changes(
        TreeUpdate {
            nodes: vec![(LocalId(2), camera)],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        &mut handler,
    );
    let calls = crate::filters::take_filter_calls();
    assert_eq!(handler.exposure.updated.len(), 2 + CONTROLS as usize);
    assert_eq!(handler.exposure.parents.len(), 2 + CONTROLS as usize);
    assert!(
        handler.exposure.children_events.is_empty(),
        "panning preserves the exposed child edges"
    );
    // The original repeated parent checks made 4,150 calls for this exact input.
    // Deduplication reduces that to 3,112 without dropping exposed controls.
    assert!(calls <= 3_200, "camera filter work regressed: {calls}");
}
