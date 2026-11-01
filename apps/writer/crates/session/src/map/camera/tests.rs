use super::*;
#[test]
fn fit_contains_world_bounds_and_zoom_preserves_the_viewport_center() {
    let mut camera = Camera::default();
    let viewport = (800., 600.);
    let bounds = (100., -20., 1200., 800.);
    camera.fit(viewport, bounds);
    assert!(camera.x + bounds.0 * camera.zoom >= 0.);
    assert!(camera.y + bounds.1 * camera.zoom >= 0.);
    assert!(camera.x + (bounds.0 + bounds.2) * camera.zoom <= viewport.0);
    assert!(camera.y + (bounds.1 + bounds.3) * camera.zoom <= viewport.1);
    let center = (
        (viewport.0 / 2. - camera.x) / camera.zoom,
        (viewport.1 / 2. - camera.y) / camera.zoom,
    );
    camera.zoom_to(1.2, viewport);
    assert!((center.0 - (viewport.0 / 2. - camera.x) / camera.zoom).abs() < 0.01);
    assert!((center.1 - (viewport.1 / 2. - camera.y) / camera.zoom).abs() < 0.01);
    assert!(camera.framing == Framing::Free);
    camera.fit(viewport, (0., 0., 100_000., 100_000.));
    assert!(100_000. * camera.zoom <= viewport.0);
    assert!(100_000. * camera.zoom <= viewport.1);
}

#[test]
fn entry_framing_keeps_working_size_when_the_viewport_changes() {
    let mut camera = Camera::default();
    for viewport in [(1200., 900.), (480., 600.)] {
        camera.frame_entry(viewport, (440., 12., 320.));
        assert_eq!(camera.zoom, 1.);
        assert_eq!(camera.x + 440. + 160., viewport.0 / 2.);
        assert_eq!(camera.y + 12., 32.);
    }
}

#[test]
fn continuous_zoom_keeps_the_point_under_the_pointer_stationary() {
    let mut camera = Camera {
        x: -120.,
        y: 80.,
        zoom: 0.7,
        framing: Framing::Free,
    };
    let anchor = (310., 210.);
    let world = (
        (anchor.0 - camera.x) / camera.zoom,
        (anchor.1 - camera.y) / camera.zoom,
    );
    for zoom in [0.73, 0.82, 1.1, 0.61] {
        camera.zoom_at(zoom, anchor);
        assert!((camera.x + world.0 * camera.zoom - anchor.0).abs() < 0.001);
        assert!((camera.y + world.1 * camera.zoom - anchor.1).abs() < 0.001);
    }
}
