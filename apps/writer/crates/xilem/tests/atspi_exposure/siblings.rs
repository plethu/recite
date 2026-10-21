use super::*;
use xilem::masonry::accesskit::Orientation;

#[test]
fn empty_transparent_and_pruned_siblings_never_hide_a_reachable_scrollbar()
-> Result<(), Box<dyn std::error::Error>> {
    for reverse in [false, true] {
        let mut window = Node::new(Role::Window);
        let children = if reverse { [2, 4, 3, 6] } else { [2, 3, 4, 6] };
        window.set_children(children.map(LocalId).to_vec());
        let mut editor = Node::new(Role::TextInput);
        editor.set_label("Manuscript");
        let empty = Node::new(Role::GenericContainer);
        let mut hidden = Node::new(Role::Group);
        hidden.set_hidden();
        hidden.set_children(vec![LocalId(5)]);
        let mut scrollbar = Node::new(Role::ScrollBar);
        scrollbar.set_orientation(Orientation::Vertical);
        scrollbar.set_label("Outer scrollbar");
        let registration = Registration::default();
        let mut adapter = Adapter::new(
            &AppContext::new(None),
            registration.clone(),
            TreeUpdate {
                nodes: vec![
                    (LocalId(0), window),
                    (LocalId(2), editor),
                    (LocalId(3), empty),
                    (LocalId(4), hidden),
                    (LocalId(5), Node::new(Role::Button)),
                    (LocalId(6), scrollbar),
                ],
                tree: Some(Tree::new(LocalId(0))),
                tree_id: TreeId::ROOT,
                focus: LocalId(6),
            },
            true,
            WindowBounds::default(),
            Actions,
        );
        for focus in [6, 2, 0] {
            adapter.update(TreeUpdate {
                nodes: vec![],
                tree: None,
                tree_id: TreeId::ROOT,
                focus: LocalId(focus),
            });
            let parent = adapter.platform_node(adapter.root_id());
            let children: Vec<_> = parent.map_children(|id| id)?;
            assert_eq!(children.len(), 2, "reverse={reverse}, focus={focus}");
            for (index, (id, expected)) in children
                .iter()
                .zip(["Manuscript", "Outer scrollbar"])
                .enumerate()
            {
                let child = adapter.platform_node(*id);
                assert_eq!(child.name()?, expected);
                assert_eq!(child.index_in_parent()?, i32::try_from(index)?);
                assert_eq!(parent.child_at_index(index)?, Some(*id));
            }
            let ledger = registration.observations();
            assert!(ledger.failures.is_empty(), "{:?}", ledger.failures);
            assert_eq!(ledger.interfaces.len(), children.len() + 1);
        }
    }
    Ok(())
}
