// Fixtures here are shared with frontend tests through the `test-support` feature.
#![allow(unused_imports, dead_code, clippy::unwrap_used, clippy::expect_used)]
pub use super::*;
pub use crate::{navigation, test_support as support};
pub use input::Input;
pub use interaction::Source;
pub use kurbo;
pub use recite_writer_session::map::placement::load;
pub use tokio::sync::mpsc as async_mpsc;
#[path = "lifecycle/tests.rs"]
pub mod lifecycle;
#[path = "scaling/tests.rs"]
pub mod scaling;
pub const SOURCE: &str = ":: start default\n> @ffee6p8\n  At the gate.\n? @rthh7ab\n  Continue.\n  -> later\n:: later\n> @bqyvpqq\n  At the bridge.\n-> END\n";
pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub writer: Writer,
    pub jobs: async_mpsc::UnboundedReceiver<navigation::worker::Job>,
    pub _runtime: tokio::runtime::Runtime,
}
pub fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_source(SOURCE)
}
pub fn fixture_source(source: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_project(support::project(source)?)
}
pub fn fixture_project(
    directory: tempfile::TempDir,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut writer = support::open(&directory)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.show_workspace(Workspace::Map);
    support::finish_navigation(&mut writer, &mut jobs)?;
    Ok(Fixture {
        directory,
        writer,
        jobs,
        _runtime: runtime,
    })
}

#[test]
pub fn map_prepares_accepted_graph_and_keeps_camera_work_independent_of_source_and_worker() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let original = writer.map.scene(writer).unwrap().camera;
    let graph = writer.map.scene(writer).unwrap().projection.graph.clone();
    writer.map_input(Source::Viewport, Input::Begin(Point::new(100., 100.)));
    writer.map_input(Source::Viewport, Input::Move(Point::new(160., 140.)));
    assert_eq!(writer.map.scene(writer).unwrap().camera.x, original.x + 60.);
    writer.map_input(Source::Viewport, Input::Cancel);
    assert_eq!(writer.map.scene(writer).unwrap().camera.x, original.x);
    writer.map_input(
        Source::Viewport,
        Input::Zoom {
            factor: 1.2,
            anchor: Point::new(100., 100.),
        },
    );
    assert_eq!(writer.map.scene(writer).unwrap().camera.zoom, 1.2);
    assert!(Arc::ptr_eq(
        &graph,
        &writer.map.scene(writer).unwrap().projection.graph
    ));
    assert!(fixture.jobs.try_recv().is_err());
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}
#[test]
pub fn move_previews_do_not_rebuild_graph_and_cancellation_does_not_write_placement() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let graph = writer.map.scene(writer).unwrap().projection.graph.clone();
    let old = graph.nodes[0].clone();
    writer.map_input(
        Source::Grip("start".into()),
        Input::Begin(Point::new(0., 0.)),
    );
    writer.map_input(
        Source::Grip("start".into()),
        Input::Move(Point::new(100., 80.)),
    );
    assert_eq!(writer.map.scene(writer).unwrap().node(0).x, old.x + 100.);
    assert!(Arc::ptr_eq(
        &graph,
        &writer.map.scene(writer).unwrap().projection.graph
    ));
    writer.map_input(Source::Grip("start".into()), Input::Cancel);
    assert_eq!(writer.map.scene(writer).unwrap().node(0).x, old.x);
    assert!(
        writer
            .map
            .scene(writer)
            .unwrap()
            .projection
            .positions
            .is_empty()
    );
    assert!(fixture.jobs.try_recv().is_err());
}
#[test]
pub fn placement_refresh_keeps_newer_typing_and_camera_and_uses_no_source_undo_step() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.map.workspace = Workspace::Split;
    let old = writer.map.scene(writer).unwrap().projection.graph.nodes[0].clone();
    writer.map_input(
        Source::Grip("start".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    writer.change(|model| {
        model.set_draft("Newer manuscript text".into());
        Ok(())
    });
    writer.zoom_map(1.2);
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().draft(), "Newer manuscript text");
    assert!(!writer.session.model().document().can_undo());
    assert_eq!(
        writer.map.scene(writer).unwrap().projection.graph.nodes[0].x,
        old.x + 24.
    );
    assert_eq!(writer.map.scene(writer).unwrap().camera.zoom, 1.2);
}

#[test]
pub fn composition_defers_automatic_refresh_without_losing_its_pending_request() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.invalidate_map();
    let pending = writer.map.refresh.clone();
    let status = writer.status.clone();
    let preedit = writer.attention.begin_preedit();
    writer.refresh_map_if_needed();
    assert!(pending.is_some());
    assert_eq!(writer.map.refresh, pending);
    assert_eq!(writer.status, status);
    assert!(fixture.jobs.try_recv().is_err());
    drop(preedit);
    writer.refresh_map_if_needed();
    assert!(writer.map.refresh.is_none());
}

#[test]
pub fn completed_placement_write_is_reconciled_when_a_newer_script_choice_cancels_display() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let file = recite_config::UserStateFile::new(fixture.directory.path().join("layouts.json"));
    writer.map.store = Some(Ok(file.clone()));
    let scene = writer.prepare_map(Purpose::Placement).scene;
    writer.map_input(
        Source::Grip("start".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    let completed = fixture.jobs.try_recv().unwrap().execute();
    let request = completed.request.clone();
    writer.show_workspace(Workspace::Script);
    drop(writer.accept_navigation(completed));
    writer.navigation_retired(&request);
    assert_eq!(writer.map.workspace, Workspace::Script);
    let cached = writer.map.scene(writer).unwrap();
    assert_eq!(*cached.projection.positions, load(&file, &scene).unwrap());
    assert!(cached.placement.is_none());
    let saved_x = cached.projection.graph.nodes[0].x;
    writer.show_workspace(Workspace::Map);
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    writer.map_input(
        Source::Grip("start".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.map.scene(writer).unwrap().node(0).x, saved_x + 24.);
    assert_eq!(writer.session.model().document().source(), SOURCE);
}

#[test]
pub fn script_choice_supersedes_pending_map_preparation() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.show_workspace(Workspace::Split);
    let completed = fixture.jobs.try_recv().unwrap().execute();
    let request = completed.request.clone();
    writer.show_workspace(Workspace::Script);
    drop(writer.accept_navigation(completed));
    writer.navigation_retired(&request);
    assert_eq!(writer.map.workspace, Workspace::Script);
}

#[test]
pub fn active_gesture_keeps_mount_membership_and_zoom_cancels_its_old_coordinate_origin() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let before = writer
        .map
        .scene(writer)
        .unwrap()
        .mounted_indices()
        .collect::<Vec<_>>();
    writer.map_input(Source::Grip("start".into()), Input::Begin(Point::ORIGIN));
    writer.map_input(
        Source::Grip("start".into()),
        Input::Move(Point::new(10_000., 10_000.)),
    );
    assert_eq!(
        writer
            .map
            .scene(writer)
            .unwrap()
            .mounted_indices()
            .collect::<Vec<_>>(),
        before
    );
    writer.zoom_map(1.2);
    assert!(matches!(
        writer.map.scene(writer).unwrap().gesture,
        Gesture::Idle
    ));
    writer.map_input(
        Source::Grip("start".into()),
        Input::Move(Point::new(20_000., 20_000.)),
    );
    assert_eq!(
        writer.map.scene(writer).unwrap().node(0),
        &writer.map.scene(writer).unwrap().projection.graph.nodes[0]
    );
    assert!(fixture.jobs.try_recv().is_err());
}
