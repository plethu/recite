use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::masonry::{accesskit::Role, core::WindowEvent};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn short_enlarged_workspace_keeps_camera_controls_and_discloses_secondary_tools() {
    let mut fixture = fixture().unwrap();
    fixture.writer.viewport.width = 960.;
    fixture.writer.viewport.height = 461.;
    let mut config = fixture.writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    fixture.writer.configure(config);
    let original = fixture
        .writer
        .session
        .model()
        .document()
        .source()
        .to_owned();
    let mut mounted = Mounted::new(fixture.writer, surface);
    mounted
        .harness
        .process_window_event(WindowEvent::Resize((960, 461).into()));
    mounted.settle();
    let bounds = |name| {
        let widget = mounted.harness.get_widget_with_id(mounted.find(name));
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
    };
    assert_eq!(
        bounds("Split unavailable")
            .intersect(bounds("Try scene"))
            .area(),
        0.,
        "mode and scene actions must not overlap"
    );
    assert!(
        mounted.state.map.scene(&mounted.state).unwrap().viewport.1 >= 120.,
        "compact controls must leave a usable logical canvas"
    );
    let fit = bounds("Fit");
    assert!(fit.x0 >= 0. && fit.x1 <= 960. && fit.y1 <= 461.);
    mounted.activate("Map tools");
    mounted.find("Find a beat");
    mounted.find("Arrange");
    mounted.activate("Help");
    assert!(mounted.state.map.scene(&mounted.state).unwrap().tools == Tools::Help);
    mounted.activate("Hide map tools");
    assert!(mounted.state.map.scene(&mounted.state).unwrap().tools == Tools::Closed);
    assert_eq!(mounted.state.session.model().document().source(), original);
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
    mounted.activate("Arrange");
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

#[test]
fn map_search_reveals_a_named_beat_and_keeps_native_typing_focus() {
    let fixture = fixture().unwrap();
    let mut mounted = Mounted::new(fixture.writer, surface);
    let field = mounted.find("Find a beat");
    mounted.type_text("Find a beat", "Later");
    assert_eq!(mounted.state.map_selected(), Some("later"));
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut selected = vec![];
    while let Some(node) = pending.pop() {
        if node.role() == Role::ListItem && node.is_selected() == Some(true) {
            selected.push(node.label().unwrap_or_default());
        }
        pending.extend(node.children());
    }
    assert_eq!(selected, ["Later · end"]);
    assert_eq!(mounted.find("Find a beat"), field);
    assert!(
        mounted
            .state
            .map
            .scene(&mounted.state)
            .unwrap()
            .focus
            .is_none()
    );
    assert_eq!(mounted.state.session.model().document().source(), SOURCE);
    assert!(!mounted.state.session.model().document().can_undo());
}
