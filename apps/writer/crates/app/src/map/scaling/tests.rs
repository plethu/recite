//! The map zooms with the interface, as the sheet does; pans and gestures keep their meaning.
use super::*;
use kurbo::{Point, Size, Vec2};
use recite_config::WriterPresentationField;

#[test]
fn pane_resize_reveals_the_selected_card_without_resetting_zoom_or_later_pan() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let scene = writer.map_scene_mut().unwrap();
    let selected = scene.projection.graph.blocks.last().unwrap().id.clone();
    scene.select(Some(selected));
    scene.camera.zoom = 0.8;
    scene.camera.framing = Framing::Free;
    writer.resize_map(Size::new(420., 300.));
    let scene = writer.map.scene(writer).unwrap();
    let index = scene.projection.graph.ids[scene.selected.as_ref().unwrap()];
    let bounds = scene.overview_bounds(index);
    assert!(bounds.x0 >= 0. && bounds.x1 <= 420.);
    assert!(bounds.y0 >= 0. && bounds.y1 <= 300.);
    assert_eq!(scene.camera.zoom, 0.8);
    writer.map_input(Source::Viewport, Input::Pan(Vec2::new(-700., -500.)));
    let panned = writer.map.scene(writer).unwrap().camera;
    writer.resize_map(Size::new(420., 300.));
    assert!(writer.map.scene(writer).unwrap().camera == panned);
}

#[test]
fn the_map_zooms_with_the_interface_and_pans_and_gestures_keep_their_meaning() {
    let mut observed = vec![];
    for scale in [100, 200] {
        let factor = f32::from(scale) / 100.;
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config);
        writer.resize_map(Size::new(800., 500.));
        let camera = Camera {
            zoom: 0.75 * factor,
            x: 40.,
            y: 30.,
            framing: Framing::Free,
            scale: factor,
        };
        writer.map_scene_mut().unwrap().camera = camera;
        let initial = writer.map.scene(writer).unwrap().overview_bounds(0);
        writer.map_input(Source::Viewport, Input::Pan(Vec2::new(30., 20.)));
        let panned = writer.map.scene(writer).unwrap().camera;
        assert_eq!((panned.x, panned.y), (70., 50.));
        let anchor = Point::new(300., 200.);
        writer.config.writer.zoom_to_pointer = true;
        writer.map_input(
            Source::Viewport,
            Input::Zoom {
                factor: 1.2,
                anchor,
            },
        );
        let zoomed = writer.map.scene(writer).unwrap().camera;
        for (point, before, after) in [
            (anchor.x as f32, panned.x, zoomed.x),
            (anchor.y as f32, panned.y, zoomed.y),
        ] {
            assert!(((point - before) / panned.zoom - (point - after) / zoomed.zoom).abs() < 0.001);
        }
        let id = writer.map.scene(writer).unwrap().projection.graph.blocks[0]
            .id
            .clone();
        let before = writer.map.scene(writer).unwrap().node(0);
        let origin = (before.x, before.y);
        writer.map_input(Source::Grip(id), Input::Begin(Point::new(100., 100.)));
        writer.map_input(Source::Viewport, Input::Move(Point::new(145., 127.)));
        let moved = writer.map.scene(writer).unwrap().node(0);
        let movement = (moved.x - origin.0, moved.y - origin.1);
        assert!((movement.0 - 45. / zoomed.zoom).abs() < 0.001);
        assert!((movement.1 - 27. / zoomed.zoom).abs() < 0.001);
        writer.cancel_map_gesture();
        observed.push((initial, zoomed.relative(), movement.0 * zoomed.zoom));
    }
    // The same relative zoom draws the card twice as large at 200%, and a drag moves it as far
    // on screen at either scale.
    let ((small, relative, moved), (large, zoomed, moved_large)) = (observed[0], observed[1]);
    assert!((large.width() - 2. * small.width()).abs() < 0.01);
    assert!((relative - zoomed).abs() < 0.001);
    assert!((moved - moved_large).abs() < 0.001);
}

#[test]
fn interface_preference_change_cancels_a_gesture_and_zooms_the_camera_with_the_interface() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.resize_map(Size::new(800., 500.));
    let camera = Camera {
        zoom: 0.8,
        x: 40.,
        y: 30.,
        framing: Framing::Free,
        scale: 1.,
    };
    writer.map_scene_mut().unwrap().camera = camera;
    writer.map_input(Source::Viewport, Input::Begin(Point::new(100., 100.)));
    writer.map_input(Source::Viewport, Input::Move(Point::new(140., 120.)));
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let scene = writer.map.scene(writer).unwrap();
    let mut zoomed = camera;
    zoomed.rescale(2.);
    assert!(scene.camera == zoomed);
    assert_eq!(scene.camera.relative(), 0.8);
    assert_eq!(scene.viewport, (800., 500.));
    assert!(matches!(scene.gesture, Gesture::Idle));
    writer.resize_map(Size::new(700., 400.));
    assert_eq!(writer.map.scene(writer).unwrap().viewport, (700., 400.));
}
