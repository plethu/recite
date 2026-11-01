use super::*;

struct Pruned {
    adapter: Adapter,
    registration: Registration,
    viewport: Node,
    branch: Node,
}
impl Pruned {
    fn new(focused: LocalId) -> Self {
        let mut window = Node::new(Role::Window);
        window.set_children(vec![LocalId(1)]);
        let mut viewport = Node::new(Role::ScrollView);
        viewport.set_clips_children();
        viewport.set_bounds(Rect::new(0., 0., 100., 100.));
        viewport.set_children(vec![LocalId(2)]);
        let mut branch = Node::new(Role::ScrollView);
        branch.set_clips_children();
        branch.set_bounds(Rect::new(0., 400., 100., 500.));
        branch.set_children(vec![LocalId(3)]);
        let mut transparent = Node::new(Role::GenericContainer);
        transparent.set_children(vec![LocalId(4), LocalId(6)]);
        let mut input = Node::new(Role::TextInput);
        input.set_bounds(Rect::new(0., 400., 100., 440.));
        input.set_children(vec![LocalId(5)]);
        let mut run = Node::new(Role::TextRun);
        run.set_value("Draft text");
        run.set_character_lengths(vec![1; 10]);
        let mut button = Node::new(Role::Button);
        button.set_bounds(Rect::new(0., 450., 100., 490.));
        let registration = Registration::default();
        let adapter = Adapter::new(
            &AppContext::new(None),
            registration.clone(),
            TreeUpdate {
                nodes: vec![
                    (LocalId(0), window),
                    (LocalId(1), viewport.clone()),
                    (LocalId(2), branch.clone()),
                    (LocalId(3), transparent),
                    (LocalId(4), input),
                    (LocalId(5), run),
                    (LocalId(6), button),
                ],
                tree: Some(Tree::new(LocalId(0))),
                tree_id: TreeId::ROOT,
                focus: focused,
            },
            true,
            WindowBounds::default(),
            Actions,
        );
        Self {
            adapter,
            registration,
            viewport,
            branch,
        }
    }
    fn update(&mut self, nodes: Vec<(LocalId, Node)>, focus: LocalId) {
        self.registration.observations().edges.clear();
        self.adapter.update(TreeUpdate {
            nodes,
            tree: None,
            tree_id: TreeId::ROOT,
            focus,
        });
    }
    fn assert_interfaces(&self, count: usize) -> Result<(), Box<dyn std::error::Error>> {
        let ledger = self.registration.observations();
        assert!(
            ledger.failures.is_empty(),
            "interface protocol violations: {:?}",
            ledger.failures
        );
        assert_eq!(ledger.interfaces.len(), count);
        for (id, interfaces) in &ledger.interfaces {
            assert_eq!(*interfaces, self.adapter.platform_node(*id).interfaces()?);
        }
        Ok(())
    }
}

#[test]
fn removing_a_pruned_branch_never_unregisters_its_unreachable_transparent_descendants()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Pruned::new(LocalId(0));
    fixture.assert_interfaces(2)?;
    fixture.viewport.clear_children();
    fixture.update(vec![(LocalId(1), fixture.viewport.clone())], LocalId(0));
    fixture.assert_interfaces(2)?;
    Ok(())
}

#[test]
fn revealing_hiding_and_removing_a_branch_reconciles_every_text_and_action_interface()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Pruned::new(LocalId(0));
    fixture.assert_interfaces(2)?;
    fixture.viewport.set_bounds(Rect::new(0., 0., 100., 500.));
    fixture.update(vec![(LocalId(1), fixture.viewport.clone())], LocalId(0));
    fixture.assert_interfaces(5)?;
    fixture.branch.set_hidden();
    fixture.update(vec![(LocalId(2), fixture.branch.clone())], LocalId(0));
    fixture.assert_interfaces(2)?;
    fixture.viewport.clear_children();
    fixture.update(vec![(LocalId(1), fixture.viewport.clone())], LocalId(0));
    fixture.assert_interfaces(2)?;
    Ok(())
}

#[test]
fn a_focused_descendant_of_a_pruned_ancestor_stays_unreachable_when_focus_moves()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Pruned::new(LocalId(4));
    fixture.assert_interfaces(2)?;
    fixture.update(Vec::new(), LocalId(0));
    fixture.assert_interfaces(2)?;
    Ok(())
}
