use super::*;

#[test]
fn large_neighbourhood_keeps_selection_and_counts_hidden_beats()
-> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::workload::source(300, 0, 1);
    let document = recite_writer_model::Document::new(&source)?;
    let blocks = document.script_snapshot()?;
    let graph = Topology::new(&blocks, &Default::default());
    let scope = nearby(&blocks, &graph, Some("beat_150")).ok_or("scope")?;
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
