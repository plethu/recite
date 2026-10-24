//! Interface preferences cannot change the meaning of an explicit camera zoom or logical gesture.
use super::*;
use recite_config::WriterPresentationField;
use xilem::masonry::kurbo::{Point, Size, Vec2};

#[test]
fn camera_geometry_pan_placement_and_pointer_zoom_are_independent_of_interface_scale() {
    let mut observed = vec![];
    for scale in [100, 200] {
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
            zoom: 0.75,
            x: 40.,
            y: 30.,
            framing: Framing::Free,
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
        observed.push((initial, panned, zoomed, movement));
    }
    assert!(observed[0] == observed[1]);
}

#[test]
fn interface_preference_change_cancels_a_gesture_without_inventing_a_viewport_or_zoom() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.resize_map(Size::new(800., 500.));
    let camera = Camera {
        zoom: 0.8,
        x: 40.,
        y: 30.,
        framing: Framing::Free,
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
    assert!(scene.camera == camera);
    assert_eq!(scene.viewport, (800., 500.));
    assert!(matches!(scene.gesture, Gesture::Idle));
    writer.resize_map(Size::new(700., 400.));
    assert_eq!(writer.map.scene(writer).unwrap().viewport, (700., 400.));
}
