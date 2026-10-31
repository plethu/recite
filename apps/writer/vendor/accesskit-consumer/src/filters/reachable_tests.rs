//! Differential exposure and fixed-input cost regressions for the AT-SPI traversal policy.
use super::*;
use crate::{Tree, TreeChangeHandler, tests::nid};
use accesskit::{
    Affine, Node as Data, NodeId, Rect, Role, Tree as TreeData, TreeId, TreeUpdate, Uuid,
};
use alloc::{vec, vec::Vec};

fn reference(node: &Node) -> FilterResult {
    let result = common_filter(node);
    if result == FilterResult::ExcludeSubtree {
        return result;
    }
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if common_filter(&ancestor) == FilterResult::ExcludeSubtree {
            return FilterResult::ExcludeSubtree;
        }
        parent = ancestor.parent();
    }
    result
}
struct Changes;
impl TreeChangeHandler for Changes {
    fn node_added(&mut self, _: &Node) {}
    fn node_updated(&mut self, _: &Node, _: &Node) {}
    fn node_removed(&mut self, _: &Node) {}
    fn focus_moved(&mut self, _: Option<&Node>, _: Option<&Node>) {}
}
fn corpus(depth: u64, variant: u64, step: u64) -> TreeUpdate {
    let mut root = Data::new(Role::Window);
    root.set_children(vec![NodeId(1)]);
    let mut nodes = vec![(NodeId(0), root)];
    for index in 1..=depth {
        let mut node = Data::new(if index == 1 {
            Role::ScrollView
        } else if variant & 1 == 0 {
            Role::GenericContainer
        } else {
            Role::Group
        });
        if index == 1 || (variant & 2 != 0 && index == depth / 2) {
            node.set_clips_children();
            node.set_bounds(Rect::new(0., 0., 100., 100.));
        }
        if variant & 4 != 0 && index == depth {
            node.set_hidden();
        }
        if index == depth / 2 {
            node.set_transform(Affine::translate((step as f64 * 15., step as f64 * -20.)));
        }
        node.set_children(if index == depth {
            (1000..1008).map(NodeId).collect()
        } else {
            vec![NodeId(index + 1)]
        });
        nodes.push((NodeId(index), node));
    }
    if variant & 8 != 0 {
        nodes[0].1.set_children(vec![NodeId(1), NodeId(9000)]);
        let mut graft = Data::new(Role::GenericContainer);
        graft.set_tree_id(TreeId(Uuid::from_u128(42)));
        nodes.push((NodeId(9000), graft));
    }
    for index in 1000..1008 {
        let mut node = Data::new(Role::Button);
        let y = ((index - 1000) as f64 - 2.) * 50.;
        if variant & 16 == 0 || index % 2 == 0 {
            node.set_bounds(Rect::new(0., y, 100., y + if step == 3 { 0. } else { 50. }));
        }
        node.set_children(vec![NodeId(index + 1000)]);
        nodes.push((NodeId(index), node));
        let mut child = Data::new(if step % 2 == 0 {
            Role::GenericContainer
        } else {
            Role::TextRun
        });
        child.set_bounds(Rect::new(0., y, 100., y + 50.));
        nodes.push((NodeId(index + 1000), child));
    }
    if step % 2 != 0 {
        nodes
            .iter_mut()
            .find(|(id, _)| *id == NodeId(depth))
            .unwrap()
            .1
            .set_children((1000..1008).rev().map(NodeId).collect::<Vec<_>>());
    }
    TreeUpdate {
        nodes,
        tree: Some(TreeData::new(NodeId(0))),
        tree_id: TreeId::ROOT,
        focus: if step % 2 == 0 {
            NodeId(0)
        } else {
            NodeId(2007)
        },
    }
}
fn compare(tree: &Tree, ids: &[NodeId]) {
    // Memoized results must not depend on query order.
    let forward = ReachableFilter::new();
    let backward = ReachableFilter::new();
    for id in ids.iter().rev() {
        backward.filter(&tree.state().node_by_id(nid(*id)).unwrap());
    }
    for id in ids {
        let node = tree.state().node_by_id(nid(*id)).unwrap();
        let expected = reference(&node);
        assert_eq!(common_filter_reachable(&node), expected, "node {id:?}");
        assert_eq!(forward.filter(&node), expected, "cached node {id:?}");
        assert_eq!(
            backward.filter(&node),
            expected,
            "reverse cached node {id:?}"
        );
        assert_eq!(
            forward.bounding_box(&node),
            node.bounding_box(),
            "node {id:?}"
        );
    }
}
#[test]
fn reachable_filter_matches_the_original_wrapper_across_geometry_and_focus_updates() {
    for depth in [1, 4, 12, 24] {
        for variant in 0..32 {
            let initial = corpus(depth, variant, 0);
            let ids = initial.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            let mut tree = Tree::new(initial, true);
            compare(&tree, &ids);
            for step in 1..4 {
                let mut update = corpus(depth, variant, step);
                update.tree = None;
                tree.update_and_process_changes(update, &mut Changes);
                compare(&tree, &ids);
            }
        }
    }
}
#[test]
fn reachable_filter_keeps_scroll_neighbours_and_excludes_deeper_offscreen_branches() {
    let tree = Tree::new(corpus(1, 0, 0), true);
    for (index, expected) in [
        (1000, FilterResult::ExcludeSubtree),
        (1001, FilterResult::Include),
        (1002, FilterResult::Include),
        (1004, FilterResult::Include),
        (1005, FilterResult::ExcludeSubtree),
        (2000, FilterResult::ExcludeSubtree),
        (2002, FilterResult::ExcludeNode),
    ] {
        assert_eq!(
            common_filter_reachable(&tree.state().node_by_id(nid(NodeId(index))).unwrap()),
            expected,
            "node {index}"
        );
    }
}

#[test]
fn reachable_filter_does_not_share_visible_ancestry_with_hidden_sibling_branches() {
    for transparent in [false, true] {
        let mut update = corpus(1, 0, 0);
        for (id, node) in &mut update.nodes {
            if *id == NodeId(1002) || *id == NodeId(1003) {
                node.set_hidden();
            }
            if transparent && *id == NodeId(1002) {
                node.set_role(Role::GenericContainer);
            }
            if transparent && *id == NodeId(2002) {
                node.set_role(Role::Button);
            }
        }
        let ids = update.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let tree = Tree::new(update, true);
        compare(&tree, &ids);
        let offscreen = tree.state().node_by_id(nid(NodeId(1001))).unwrap();
        assert_eq!(
            common_filter_reachable(&offscreen),
            FilterResult::ExcludeSubtree
        );
    }
}
#[test]
fn reachable_filter_does_not_repeat_recursive_base_checks_for_a_deep_visible_chain() {
    let mut update = corpus(32, 1, 0);
    update.nodes[1].1.clear_clips_children();
    update.nodes[1].1.clear_bounds();
    let tree = Tree::new(update, true);
    let node = tree.state().node_by_id(nid(NodeId(2002))).unwrap();
    take_base_checks();
    let expected = reference(&node);
    let old = take_base_checks();
    let actual = common_filter_reachable(&node);
    let next = take_base_checks();
    assert_eq!(actual, expected);
    // This exact workload made 1,157 base checks through the original wrapper.
    // The reachable traversal makes 68, including the structural parent queries.
    assert!(old > 1_000);
    assert!(next <= 72, "deep exposure work regressed: {next}");
}

#[test]
fn reachable_filter_checks_visible_inheritance_once_as_depth_grows() {
    for depth in [16, 32, 64] {
        let mut update = corpus(depth, 1, 0);
        update.nodes[1].1.clear_clips_children();
        update.nodes[1].1.clear_bounds();
        let tree = Tree::new(update, true);
        let node = tree.state().node_by_id(nid(NodeId(2002))).unwrap();
        take_hidden_ancestor_visits();
        let expected = reference(&node);
        let old = take_hidden_ancestor_visits();
        let actual = common_filter_reachable(&node);
        let visits = take_hidden_ancestor_visits();
        assert_eq!(actual, expected);
        assert_eq!(visits, usize::try_from(depth).unwrap() + 3);
        assert!(old > visits * 100);
        std::println!("depth {depth}: hidden ancestor visits {old} -> {visits}");
    }
}

#[test]
fn reachable_filter_preserves_hidden_focus_and_active_descendant_exceptions() {
    for hidden in [NodeId(0), NodeId(1), NodeId(4), NodeId(1002)] {
        for focus in [NodeId(0), NodeId(1), NodeId(4), NodeId(1002), NodeId(2002)] {
            for active_descendant in [false, true] {
                let mut update = corpus(4, 1, 0);
                update
                    .nodes
                    .iter_mut()
                    .find(|(id, _)| *id == hidden)
                    .unwrap()
                    .1
                    .set_hidden();
                update.focus = focus;
                if active_descendant {
                    update.nodes[4].1.set_active_descendant(NodeId(1002));
                }
                let ids = update.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
                compare(&Tree::new(update, true), &ids);
            }
        }
    }
    let mut update = corpus(4, 1, 0);
    update.nodes[4].1.set_hidden();
    update.focus = NodeId(4);
    let tree = Tree::new(update, true);
    let parent = tree.state().node_by_id(nid(NodeId(4))).unwrap();
    let child = tree.state().node_by_id(nid(NodeId(1002))).unwrap();
    assert_eq!(common_filter_reachable(&parent), FilterResult::Include);
    assert_eq!(
        common_filter_reachable(&child),
        FilterResult::ExcludeSubtree
    );
}

#[test]
fn reachable_filter_preserves_visibility_across_attached_grafts() {
    for hidden in [false, true] {
        for focused in [false, true] {
            let mut update = corpus(4, 9, 0);
            let ids = update.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            if hidden {
                update
                    .nodes
                    .iter_mut()
                    .find(|(id, _)| *id == NodeId(9000))
                    .unwrap()
                    .1
                    .set_hidden();
            }
            let mut tree = Tree::new(update, true);
            let mut root = Data::new(Role::Group);
            root.set_children(vec![NodeId(1)]);
            tree.update_and_process_changes(
                TreeUpdate {
                    nodes: vec![(NodeId(0), root), (NodeId(1), Data::new(Role::Button))],
                    tree: Some(TreeData::new(NodeId(0))),
                    tree_id: TreeId(Uuid::from_u128(42)),
                    focus: NodeId(1),
                },
                &mut Changes,
            );
            if focused {
                tree.update_and_process_changes(
                    TreeUpdate {
                        nodes: vec![],
                        tree: None,
                        tree_id: TreeId::ROOT,
                        focus: NodeId(9000),
                    },
                    &mut Changes,
                );
            }
            compare(&tree, &ids);
            let graft = tree.state().node_by_id(nid(NodeId(9000))).unwrap();
            let root = graft.children().next().unwrap();
            let child = root.children().next().unwrap();
            for node in [root, child] {
                assert_eq!(common_filter_reachable(&node), reference(&node));
            }
        }
    }
}
#[test]
fn cached_filter_checks_each_node_once_across_a_deep_region() {
    let depth = 64;
    let update = corpus(depth, 1, 1);
    let ids = update.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let tree = Tree::new(update, true);
    take_base_checks();
    let filter = ReachableFilter::new();
    for id in &ids {
        filter.filter(&tree.state().node_by_id(nid(*id)).unwrap());
    }
    let cached = take_base_checks();
    for id in &ids {
        common_filter_reachable(&tree.state().node_by_id(nid(*id)).unwrap());
    }
    let direct = take_base_checks();
    // One base check per node, independent of depth; the direct filter repeats
    // the ancestor chain for every node.
    assert!(
        cached <= ids.len(),
        "{cached} checks for {} nodes",
        ids.len()
    );
    assert!(direct > cached * 10, "direct {direct}, cached {cached}");
}
