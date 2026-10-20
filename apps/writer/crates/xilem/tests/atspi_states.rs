use accesskit_atspi_common::{
    Adapter, AdapterCallback, AppContext, Event, InterfaceSet, NodeId, State, WindowBounds,
};
use xilem::masonry::accesskit::{
    ActionHandler, ActionRequest, Node, NodeId as LocalNodeId, Role, Tree, TreeId, TreeUpdate,
};

struct Callback;
impl AdapterCallback for Callback {
    fn register_interfaces(&self, _: &Adapter, _: NodeId, _: InterfaceSet) {}
    fn unregister_interfaces(&self, _: &Adapter, _: NodeId, _: InterfaceSet) {}
    fn emit_event(&self, _: &Adapter, _: Event) {}
}

struct Actions;
impl ActionHandler for Actions {
    fn do_action(&mut self, _: ActionRequest) {}
}

#[test]
fn disabled_native_controls_drop_enabled_and_sensitive_states_without_losing_their_identity() {
    for (role, read_only) in [
        (Role::Button, false),
        (Role::TextInput, false),
        (Role::TextInput, true),
    ] {
        let app = AppContext::new(None);
        let mut window = Node::new(Role::Window);
        window.set_children(vec![LocalNodeId(1)]);
        let mut control = Node::new(role);
        if read_only {
            control.set_read_only();
        }
        let mut adapter = Adapter::new(
            &app,
            Callback,
            TreeUpdate {
                nodes: vec![(LocalNodeId(0), window), (LocalNodeId(1), control.clone())],
                tree: Some(Tree::new(LocalNodeId(0))),
                tree_id: TreeId::ROOT,
                focus: LocalNodeId(0),
            },
            false,
            WindowBounds::default(),
            Actions,
        );
        let id = adapter
            .platform_node(adapter.root_id())
            .child_at_index(0)
            .unwrap()
            .unwrap();
        let node = adapter.platform_node(id);
        for disabled in [false, true, false] {
            if disabled {
                control.set_disabled();
            } else {
                control.clear_disabled();
            }
            adapter.update(TreeUpdate {
                nodes: vec![(LocalNodeId(1), control.clone())],
                tree: None,
                tree_id: TreeId::ROOT,
                focus: LocalNodeId(0),
            });
            let states = node.state();
            assert_eq!(states.contains(State::Enabled), !disabled && !read_only);
            assert_eq!(states.contains(State::Sensitive), !disabled && !read_only);
            assert_eq!(states.contains(State::ReadOnly), !disabled && read_only);
            assert!(!states.contains(State::Defunct));
        }
    }
}
