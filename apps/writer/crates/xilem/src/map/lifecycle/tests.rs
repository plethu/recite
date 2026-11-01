use super::*;
use recite_writer_session::map::placement::load;
use xilem::masonry::kurbo;

#[test]
fn accepted_authoring_changes_refresh_visible_map_without_applying_later_typing() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.map.workspace = Workspace::Split;
    writer.change(|model| {
        model.set_draft("The gate opens.".into());
        Ok(())
    });
    writer.navigate(Target::Edit(recite_writer_model::EditOperation::Apply));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(
        writer.map.scene(writer).unwrap().projection.synopsis[0].prose,
        "The gate opens."
    );
    writer.navigate(Target::Structure(
        recite_writer_model::StructuralCommand::AddBeat,
    ));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(
        writer
            .map
            .scene(writer)
            .unwrap()
            .projection
            .graph
            .blocks
            .len(),
        3
    );
    assert_eq!(writer.map_selected(), writer.reading.selected.as_deref());
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}

#[test]
fn failed_placement_save_retains_the_move_and_retry_preserves_disk_validation() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let path = fixture.directory.path().join("layouts.json");
    std::fs::write(&path, "broken").unwrap();
    let file = recite_config::UserStateFile::new(path.clone());
    writer.map.store = Some(Ok(file.clone()));
    let scene_key = writer.prepare_map(Purpose::Placement).scene;
    writer.map_input(
        Source::Grip("start".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    let scene = writer.map.scene(writer).unwrap();
    assert!(matches!(
        scene.projection.placement,
        placement::Status::SaveFailed(_)
    ));
    assert!(!scene.projection.positions.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
    std::fs::write(&path, r#"{"version":1,"scenes":{}}"#).unwrap();
    writer.refresh_map_layout();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    let scene = writer.map.scene(writer).unwrap();
    assert!(matches!(
        scene.projection.placement,
        placement::Status::Ready
    ));
    assert_eq!(
        *scene.projection.positions,
        load(&file, &scene_key).unwrap()
    );
}

#[test]
fn direct_save_refreshes_the_map_after_its_implicit_apply() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.map.workspace = Workspace::Split;
    writer.change(|model| {
        model.set_draft("Saved manuscript becomes card prose.".into());
        Ok(())
    });
    support::save(writer).unwrap();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(
        writer.map.scene(writer).unwrap().projection.synopsis[0].prose,
        "Saved manuscript becomes card prose."
    );
    assert!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite"))
            .unwrap()
            .contains("Saved manuscript becomes card prose.")
    );
}

#[test]
fn reload_refreshes_visible_map_and_inactive_write_receipts_update_the_cached_geometry() {
    let directory = support::project(SOURCE).unwrap();
    let other = directory.path().join("other.recite");
    std::fs::write(
        &other,
        ":: other default\n> @z0g6c80\n  Another scene.\n-> END\n",
    )
    .unwrap();
    let mut fixture = fixture_project(directory).unwrap();
    let writer = &mut fixture.writer;
    writer.map.workspace = Workspace::Split;
    std::fs::write(
        fixture.directory.path().join("scene.recite"),
        SOURCE.replace("At the gate.", "An external edit."),
    )
    .unwrap();
    writer.reload();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(
        writer.map.scene(writer).unwrap().projection.synopsis[0].prose,
        "An external edit."
    );
    writer.map_input(
        Source::Grip("start".into()),
        Input::Direction(kurbo::Vec2::new(1., 0.)),
    );
    let mut prepared = writer
        .prepare_map(Purpose::Placement)
        .execute(&recite_compiler::authoring::CancellationToken::new())
        .unwrap();
    // Reconcile a result by its cached scene even while another scene is current.
    let key = prepared.key.clone();
    let old_x = writer.map.scenes[&key].projection.graph.nodes[0].x;
    writer
        .session
        .project_mut()
        .unwrap()
        .switch(&other)
        .unwrap();
    writer.reconcile_map_placement(&mut prepared);
    assert_eq!(
        writer.map.scenes[&key].projection.graph.nodes[0].x,
        old_x + 24.
    );
}
