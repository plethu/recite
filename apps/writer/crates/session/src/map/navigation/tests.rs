use super::*;
#[test]
fn directional_navigation_uses_geometry_and_reveal_preserves_zoom() {
    let nodes = vec![
        Node {
            x: 300.,
            y: 0.,
            width: 200.,
        },
        Node {
            x: 0.,
            y: 250.,
            width: 200.,
        },
        Node {
            x: 300.,
            y: 250.,
            width: 200.,
        },
    ];
    assert_eq!(nearest(&nodes, 0, (0., 1.)), Some(2));
    assert_eq!(nearest(&nodes, 2, (-1., 0.)), Some(1));
    let mut camera = Camera {
        zoom: 0.7,
        ..Camera::default()
    };
    reveal(&mut camera, &nodes[2], (200., 200.));
    assert_eq!(camera.zoom, 0.7);
    assert!(camera.x + 500. * camera.zoom <= 184.);
}
