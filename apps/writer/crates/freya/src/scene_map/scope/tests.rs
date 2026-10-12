use super::*;

#[test]
fn large_neighbourhood_keeps_selection_and_counts_hidden_beats()
-> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::workload::source(300, 0, 1);
    let document = recite_writer_model::Document::new(&source)?;
    let blocks = document.script_snapshot()?;
    let graph = Topology::new(blocks.clone(), &Default::default());
    let scope = nearby(&graph, Some("beat_150")).ok_or("scope")?;
    assert_eq!(
        scope.node_indices,
        blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| { scope.members.contains(&block.id).then_some(index) })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        scope.edge_indices,
        graph
            .links
            .iter()
            .enumerate()
            .filter_map(|(index, link)| {
                (scope.members.contains(&link.origin) && scope.members.contains(&link.destination))
                    .then_some(index)
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(scope.members.len(), 80);
    assert!(scope.members.contains("beat_150"));
    assert!(!scope.members.contains("beat_0"));
    assert!(scope.bounds.2.is_finite() && scope.bounds.3.is_finite());
    Ok(())
}

#[test]
fn culling_keeps_a_route_crossing_the_viewport() {
    let camera = crate::scene_map::camera::Camera::default();
    assert!(crate::scene_map::topology::visible(
        (-1000., 100., 3000., 20.),
        camera,
        (1000., 800.)
    ));
    assert!(!crate::scene_map::topology::visible(
        (-1000., -1000., 20., 20.),
        camera,
        (1000., 800.)
    ));
}

#[test]
fn duplicate_ids_preserve_render_membership_without_expanding_bfs_bounds()
-> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::workload::source(300, 0, 1)
        .replace(":: beat_299", ":: beat_0")
        .replacen("-> beat_1", "-> beat_0", 1);
    let document = recite_writer_model::Document::new(&source)?;
    assert!(!document.diagnostics().is_empty());
    let blocks = document.script_snapshot()?;
    let graph = Topology::new(blocks, &Default::default());
    let scope = nearby(&graph, None).ok_or("scope")?;
    assert_eq!(scope.node_indices, vec![0, 299]);
    assert_eq!(scope.edge_indices, vec![0]);
    assert_eq!(scope.members, BTreeSet::from(["beat_0".into()]));
    let first = &graph.nodes[0];
    assert_eq!(scope.bounds, (first.x, first.y, first.width, HEIGHT));
    Ok(())
}
