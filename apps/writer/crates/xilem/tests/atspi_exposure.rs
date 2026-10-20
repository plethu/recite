use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use accesskit_atspi_common::{
    Adapter, AdapterCallback, AppContext, Event, InterfaceSet, NodeId, NodeIdOrRoot, ObjectEvent,
    WindowBounds,
};
use xilem::masonry::accesskit::{
    ActionHandler, ActionRequest, Affine, Node, NodeId as LocalId, Rect, Role, Tree, TreeId,
    TreeUpdate,
};

#[derive(Default)]
struct Observations {
    registered: BTreeSet<NodeId>,
    edges: BTreeSet<(NodeId, NodeId, bool)>,
    selections: usize,
}
#[derive(Clone, Default)]
struct Registration(Arc<Mutex<Observations>>);
impl Registration {
    fn observations(&self) -> std::sync::MutexGuard<'_, Observations> {
        // A failed callback assertion already fails the test; retain its diagnostic ledger.
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
impl AdapterCallback for Registration {
    fn register_interfaces(&self, _: &Adapter, id: NodeId, interfaces: InterfaceSet) {
        if interfaces.contains(atspi_common::Interface::Accessible) {
            assert!(
                self.observations().registered.insert(id),
                "duplicate registration: {id:?}"
            );
        }
    }
    fn unregister_interfaces(&self, _: &Adapter, id: NodeId, interfaces: InterfaceSet) {
        if interfaces.contains(atspi_common::Interface::Accessible) {
            assert!(
                self.observations().registered.remove(&id),
                "duplicate removal: {id:?}"
            );
        }
    }
    fn emit_event(&self, _: &Adapter, event: Event) {
        if let Event::Object {
            target: NodeIdOrRoot::Node(parent),
            event,
        } = event
        {
            let mut ledger = self.observations();
            let edge = match event {
                ObjectEvent::ChildAdded(_, child) => {
                    assert!(
                        ledger.registered.contains(&child),
                        "child announced before registration"
                    );
                    Some((parent, child, true))
                }
                ObjectEvent::ChildRemoved(child) => Some((parent, child, false)),
                ObjectEvent::SelectionChanged => {
                    ledger.selections += 1;
                    None
                }
                _ => None,
            };
            if let Some(edge) = edge {
                assert!(ledger.edges.insert(edge), "duplicate child event");
            }
        }
    }
}
struct Actions;
impl ActionHandler for Actions {
    fn do_action(&mut self, _: ActionRequest) {}
}

struct ScrollFixture {
    adapter: Adapter,
    registration: Registration,
    viewport: Node,
    content: Node,
}
impl ScrollFixture {
    fn new() -> Self {
        let mut window = Node::new(Role::Window);
        window.set_children(vec![LocalId(1)]);
        let mut viewport = Node::new(Role::ScrollView);
        viewport.set_clips_children();
        viewport.set_bounds(Rect::new(0., 0., 100., 100.));
        viewport.set_children(vec![LocalId(2)]);
        let mut content = Node::new(Role::GenericContainer);
        content.set_children((3..9).map(LocalId).collect::<Vec<_>>());
        let mut nodes = vec![
            (LocalId(0), window),
            (LocalId(1), viewport.clone()),
            (LocalId(2), content.clone()),
        ];
        for index in 3..9 {
            let mut button = Node::new(Role::Button);
            button.set_label(format!("Control {index}"));
            let y = (index - 3) as f64 * 80.;
            button.set_bounds(Rect::new(0., y, 100., y + 40.));
            nodes.push((LocalId(index), button));
        }
        let registration = Registration::default();
        let adapter = Adapter::new(
            &AppContext::new(None),
            registration.clone(),
            TreeUpdate {
                nodes,
                tree: Some(Tree::new(LocalId(0))),
                tree_id: TreeId::ROOT,
                focus: LocalId(0),
            },
            false,
            WindowBounds::default(),
            Actions,
        );
        Self {
            adapter,
            registration,
            viewport,
            content,
        }
    }
    fn update(&mut self, nodes: Vec<(LocalId, Node)>) {
        self.registration.observations().edges.clear();
        self.adapter.update(TreeUpdate {
            nodes,
            tree: None,
            tree_id: TreeId::ROOT,
            focus: LocalId(0),
        });
    }
    fn exposed(&self) -> Result<BTreeSet<NodeId>, Box<dyn std::error::Error>> {
        let mut exposed = BTreeSet::new();
        let mut pending = vec![self.adapter.root_id()];
        while let Some(id) = pending.pop() {
            assert!(exposed.insert(id), "duplicate node in exposed tree");
            let node = self.adapter.platform_node(id);
            for index in 0..usize::try_from(node.child_count()?)? {
                pending.push(
                    node.child_at_index(index)?
                        .ok_or("missing advertised child")?,
                );
            }
        }
        Ok(exposed)
    }
    fn assert_registered(&self) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            self.exposed()?,
            self.registration.observations().registered,
            "exposed and registered objects must agree"
        );
        Ok(())
    }
}

#[test]
fn ancestor_only_scroll_registers_newly_exposed_controls_and_removes_clipped_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    fixture.assert_registered()?;
    let initial = fixture.exposed()?;
    for offset in [-320., 0.] {
        fixture
            .content
            .set_transform(Affine::translate((0., offset)));
        fixture.update(vec![(LocalId(2), fixture.content.clone())]);
        fixture.assert_registered()?;
    }
    assert_eq!(fixture.exposed()?, initial);
    Ok(())
}

#[test]
fn resizing_clip_region_reconciles_unchanged_descendants() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = ScrollFixture::new();
    let initial = fixture.exposed()?;
    fixture.viewport.set_bounds(Rect::new(0., 0., 100., 500.));
    fixture.update(vec![(LocalId(1), fixture.viewport.clone())]);
    assert!(fixture.exposed()?.len() > initial.len());
    fixture.assert_registered()?;
    Ok(())
}

#[test]
fn moving_one_control_reconciles_its_unchanged_boundary_neighbour()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    let mut moved = Node::new(Role::Button);
    moved.set_label("Control 7");
    moved.set_bounds(Rect::new(0., 0., 100., 40.));
    fixture.update(vec![(LocalId(7), moved)]);
    fixture.assert_registered()?;
    Ok(())
}

#[test]
fn nested_clipping_and_overlapping_geometry_updates_keep_registration_consistent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    fixture.content.set_children(vec![LocalId(9)]);
    let mut inner = Node::new(Role::ScrollView);
    inner.set_clips_children();
    inner.set_bounds(Rect::new(0., 0., 100., 100.));
    inner.set_children((3..9).map(LocalId).collect::<Vec<_>>());
    fixture.update(vec![
        (LocalId(2), fixture.content.clone()),
        (LocalId(9), inner.clone()),
    ]);
    fixture.assert_registered()?;
    fixture
        .content
        .set_transform(Affine::translate((0., -320.)));
    inner.set_bounds(Rect::new(0., 320., 100., 420.));
    fixture.update(vec![
        (LocalId(2), fixture.content.clone()),
        (LocalId(9), inner),
    ]);
    fixture.assert_registered()?;
    Ok(())
}

#[test]
fn inserting_and_removing_boundary_siblings_reconciles_the_retained_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    let initial = fixture.exposed()?;
    fixture.content.set_children(vec![
        LocalId(3),
        LocalId(4),
        LocalId(9),
        LocalId(5),
        LocalId(6),
        LocalId(7),
        LocalId(8),
    ]);
    let mut inserted = Node::new(Role::Button);
    inserted.set_bounds(Rect::new(0., 0., 100., 40.));
    fixture.update(vec![
        (LocalId(2), fixture.content.clone()),
        (LocalId(9), inserted),
    ]);
    fixture.assert_registered()?;
    fixture
        .content
        .set_children((3..9).map(LocalId).collect::<Vec<_>>());
    fixture.update(vec![(LocalId(2), fixture.content.clone())]);
    fixture.assert_registered()?;
    assert_eq!(fixture.exposed()?, initial);
    Ok(())
}

#[test]
fn hidden_and_transparent_parent_transitions_preserve_exposure_registration()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    fixture.content.set_hidden();
    fixture.update(vec![(LocalId(2), fixture.content.clone())]);
    fixture.assert_registered()?;
    fixture.content.clear_hidden();
    fixture.content.set_role(Role::Group);
    fixture.update(vec![(LocalId(2), fixture.content.clone())]);
    fixture.assert_registered()?;
    fixture.content.set_role(Role::GenericContainer);
    fixture.update(vec![(LocalId(2), fixture.content.clone())]);
    fixture.assert_registered()?;
    Ok(())
}

#[test]
fn host_focus_exception_registers_the_selected_clipped_item_and_emits_selection_events()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ScrollFixture::new();
    fixture.viewport.set_role(Role::ListBox);
    let mut selected = Node::new(Role::ListBoxOption);
    selected.set_selected(true);
    selected.set_bounds(Rect::new(0., 400., 100., 440.));
    fixture.adapter.update(TreeUpdate {
        nodes: vec![
            (LocalId(1), fixture.viewport.clone()),
            (LocalId(8), selected),
        ],
        tree: None,
        tree_id: TreeId::ROOT,
        focus: LocalId(8),
    });
    for focused in [true, false] {
        fixture.registration.observations().edges.clear();
        fixture.registration.observations().selections = 0;
        fixture.adapter.update_window_focus_state(focused);
        fixture.assert_registered()?;
        assert_eq!(fixture.registration.observations().selections, 1);
    }
    Ok(())
}
