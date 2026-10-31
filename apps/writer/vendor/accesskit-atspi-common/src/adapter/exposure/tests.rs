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

#[derive(Default)]
struct Ledger {
    registered: HashSet<NodeId>,
    unregistered: HashSet<NodeId>,
    events: usize,
}
#[derive(Clone, Default)]
struct Recording(Arc<Mutex<Ledger>>);
impl AdapterCallback for Recording {
    fn register_interfaces(&self, _: &Adapter, id: NodeId, _: InterfaceSet) {
        self.0.lock().unwrap().registered.insert(id);
    }
    fn unregister_interfaces(&self, _: &Adapter, id: NodeId, _: InterfaceSet) {
        self.0.lock().unwrap().unregistered.insert(id);
    }
    fn emit_event(&self, _: &Adapter, _: Event) {
        self.0.lock().unwrap().events += 1;
    }
}

/// A clipped viewport holding one transparent camera layer of stacked controls.
fn camera_region(controls: u64, spacing: f64) -> (Vec<(LocalId, NodeData)>, NodeData) {
    let mut root = NodeData::new(Role::Window);
    root.set_children(vec![LocalId(1)]);
    let mut viewport = NodeData::new(Role::ScrollView);
    viewport.set_clips_children();
    viewport.set_bounds(Rect::new(0., 0., 1000., 1000.));
    viewport.set_children(vec![LocalId(2)]);
    let mut camera = NodeData::new(Role::GenericContainer);
    camera.set_children((3..3 + controls).map(LocalId).collect::<Vec<_>>());
    let mut nodes = vec![
        (LocalId(0), root),
        (LocalId(1), viewport),
        (LocalId(2), camera.clone()),
    ];
    for index in 3..3 + controls {
        let mut button = NodeData::new(Role::Button);
        button.set_label(format!("Beat {index}"));
        let y = (index - 3) as f64 * spacing;
        button.set_bounds(Rect::new(0., y, 100., y + 100.));
        nodes.push((LocalId(index), button));
    }
    (nodes, camera)
}
fn adapter_for(nodes: Vec<(LocalId, NodeData)>, callback: Recording) -> Adapter {
    Adapter::new(
        &AppContext::new(None),
        callback,
        TreeUpdate {
            nodes,
            tree: Some(TreeData::new(LocalId(0))),
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        false,
        WindowBounds::default(),
        Actions,
    )
}
fn exposed(adapter: &Adapter) -> HashSet<NodeId> {
    let tree = adapter.context.tree.read().unwrap();
    let mut pending = vec![tree.state().root()];
    let mut exposed = HashSet::new();
    while let Some(node) = pending.pop() {
        if accesskit_consumer::common_filter_reachable(&node) == FilterResult::Include {
            exposed.insert(node.id());
        }
        pending.extend(node.children());
    }
    exposed
}
fn translate(
    adapter: &Adapter,
    mut camera: NodeData,
    offset: (f64, f64),
) -> AdapterChangeHandler<'_> {
    camera.set_transform(accesskit::Affine::translate(offset));
    let mut handler = AdapterChangeHandler::new(adapter);
    let mut tree = adapter.context.tree.write().unwrap();
    tree.update_and_process_changes(
        TreeUpdate {
            nodes: vec![(LocalId(2), camera)],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        },
        &mut handler,
    );
    handler
}

#[test]
fn camera_translation_inside_the_clip_notifies_only_the_moved_layer() {
    const CONTROLS: u64 = 128;
    let (nodes, camera) = camera_region(CONTROLS, 0.);
    let ledger = Recording::default();
    let adapter = adapter_for(nodes, ledger.clone());
    *ledger.0.lock().unwrap() = Ledger::default();
    crate::filters::take_filter_calls();
    let handler = translate(&adapter, camera, (20., 10.));
    let calls = crate::filters::take_filter_calls();
    // The region was reconciled, but only the moved layer received a full update.
    assert_eq!(handler.exposure.new_subtrees.len(), 2 + CONTROLS as usize);
    assert_eq!(handler.exposure.updated.len(), 1);
    assert!(handler.exposure.parents.is_empty());
    assert!(handler.exposure.children_events.is_empty());
    let ledger = ledger.0.lock().unwrap();
    assert!(ledger.registered.is_empty() && ledger.unregistered.is_empty());
    assert_eq!(ledger.events, 0, "no exposure or state changed");
    // Region filtering is memoized per tree state; uncached, this input took 3,112 calls.
    // The two remaining calls are the moved layer's own text-change parent lookups.
    assert!(calls <= 2, "camera filter work regressed: {calls}");
}

#[test]
fn camera_translation_across_the_clip_registers_exactly_the_changed_exposure() {
    let (nodes, camera) = camera_region(64, 150.);
    let ledger = Recording::default();
    let adapter = adapter_for(nodes, ledger.clone());
    let before = exposed(&adapter);
    *ledger.0.lock().unwrap() = Ledger::default();
    crate::filters::take_filter_calls();
    let handler = translate(&adapter, camera, (0., -2_400.));
    let calls = crate::filters::take_filter_calls();
    let queued = handler.exposure.children_events.len();
    drop(handler);
    let after = exposed(&adapter);
    let ledger = ledger.0.lock().unwrap();
    assert_eq!(
        ledger.registered,
        after.difference(&before).copied().collect()
    );
    assert_eq!(
        ledger.unregistered,
        before.difference(&after).copied().collect()
    );
    assert!(!ledger.registered.is_empty() && !ledger.unregistered.is_empty());
    assert!(queued > 0, "the viewport's exposed child edges changed");
    // Child-edge diffs reuse the per-state caches; registering and retiring the changed
    // controls accounts for the rest. Uncached child diffs took 270 calls for this input.
    assert!(calls <= 36, "exposure filter work regressed: {calls}");
}
