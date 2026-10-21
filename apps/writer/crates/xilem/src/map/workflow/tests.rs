use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_map_controls_frame_the_scene_without_mutating_source_or_authoring_history() {
    let source = (0..205)
        .map(|index| {
            format!(
                ":: beat{index:03}{}\n-> END\n",
                if index == 0 { " default" } else { "" }
            )
        })
        .collect::<String>();
    let mut fixture = fixture_source(&source).unwrap();
    let file = recite_config::UserStateFile::new(fixture.directory.path().join("layouts.json"));
    fixture.writer.map.store = Some(Ok(file.clone()));
    let mut mounted = Mounted::new(fixture.writer, surface);
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    mounted.state.navigation.sender = Some(sender);
    mounted.state.navigation.runtime = Some(fixture._runtime.handle().clone());
    mounted.activate("100%");
    assert_eq!(
        mounted.state.map.scene(&mounted.state).unwrap().camera.zoom,
        1.
    );
    mounted.activate("Zoom in");
    assert_eq!(
        mounted.state.map.scene(&mounted.state).unwrap().camera.zoom,
        1.2
    );
    mounted.activate("Zoom out");
    assert!(
        (mounted.state.map.scene(&mounted.state).unwrap().camera.zoom - 1.).abs() < f32::EPSILON
    );
    mounted.activate("Fit");
    let scene = mounted.state.map.scene(&mounted.state).unwrap();
    let (left, top, width, height) = scene.bounds();
    let camera = scene.camera;
    assert!(left * camera.zoom + camera.x >= 15.9);
    assert!(top * camera.zoom + camera.y >= 15.9);
    assert!((left + width) * camera.zoom + camera.x <= scene.viewport.0 - 15.9);
    assert!((top + height) * camera.zoom + camera.y <= scene.viewport.1 - 15.9);
    mounted.activate("Show whole scene");
    assert!(mounted.state.map.scene(&mounted.state).unwrap().whole);
    mounted.activate("Show nearby beats");
    assert!(!mounted.state.map.scene(&mounted.state).unwrap().whole);
    mounted.state.map_input(
        Source::Grip("beat000".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    support::finish_navigation(&mut mounted.state, &mut jobs).unwrap();
    let identity = mounted.state.prepare_map(Purpose::Placement).scene;
    assert!(!load(&file, &identity).unwrap().is_empty());
    mounted.settle();
    mounted.activate("Map actions");
    mounted.activate("Arrange automatically");
    support::finish_navigation(&mut mounted.state, &mut jobs).unwrap();
    assert!(load(&file, &identity).unwrap().is_empty());
    assert!(
        mounted
            .state
            .map
            .scene(&mounted.state)
            .unwrap()
            .projection
            .positions
            .is_empty()
    );
    mounted.settle();
    mounted.activate("Open script");
    support::finish_navigation(&mut mounted.state, &mut jobs).unwrap();
    assert_eq!(mounted.state.map.workspace, Workspace::Split);
    assert_eq!(
        mounted
            .state
            .session
            .model()
            .selected_block()
            .unwrap()
            .as_deref(),
        Some("beat000")
    );
    assert_eq!(mounted.state.session.model().document().source(), source);
    assert!(!mounted.state.session.model().document().can_undo());
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        source
    );
}
