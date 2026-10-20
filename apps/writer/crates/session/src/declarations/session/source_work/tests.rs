use super::super::tests::{SOURCE, session};
use super::*;
use crate::declarations::producer::Generated;
use crate::declarations::recovery::Store;
use std::fs;

#[test]
fn rebinding_the_same_source_reuses_its_live_lease_and_stays_busy_until_retirement() {
    let (directory, mut session) = session().unwrap();
    let path = directory.path().join("schema.toml");
    let work = session
        .prepare_source(SourceAction::Bind(path.clone()))
        .unwrap();
    assert!(session.busy());
    assert!(Store::open(&path).is_err());
    let mut receipt = work.execute();
    let retirement = session.accept_source(&mut receipt).unwrap();
    assert!(session.busy());
    assert!(Store::open(&path).is_err());
    session
        .accept_source_recovery(&mut retirement.execute())
        .unwrap();
    assert!(!session.busy());
    assert!(Store::open(&path).is_err());
}

#[test]
fn archived_reload_rejects_newer_typing_or_cancellation_without_losing_the_archive() {
    for cancel in [false, true] {
        let (directory, mut session) = session().unwrap();
        session.source.as_mut().unwrap().draft = "Captured unfinished TOML [".into();
        let work = session.prepare_source(SourceAction::Reload).unwrap();
        fs::write(
            directory.path().join("schema.toml"),
            SOURCE.replace("Mara", "External"),
        )
        .unwrap();
        let mut receipt = work.execute();
        let archive = receipt.archive().unwrap().to_owned();
        session.source.as_mut().unwrap().draft = "Newer unfinished TOML [".into();
        if cancel {
            session.cancel();
        }
        assert!(session.accept_source(&mut receipt).is_err());
        assert!(!session.busy());
        assert_eq!(
            session.source.as_ref().unwrap().draft,
            "Newer unfinished TOML ["
        );
        assert_eq!(
            fs::read_to_string(archive).unwrap(),
            "Captured unfinished TOML ["
        );
    }
}

#[test]
fn source_recovery_fence_is_single_owner_and_preserves_typing_after_adoption() {
    let (directory, mut session) = session().unwrap();
    let (_other_directory, mut other) = self::session().unwrap();
    session.source.as_mut().unwrap().draft = "Old unfinished [".into();
    fs::write(
        directory.path().join("schema.toml"),
        SOURCE.replace("Mara", "External"),
    )
    .unwrap();
    let mut receipt = session
        .prepare_source(SourceAction::Reload)
        .unwrap()
        .execute();
    assert!(other.accept_source(&mut receipt).is_err());
    let retirement = session.accept_source(&mut receipt).unwrap();
    assert!(session.busy());
    assert!(session.source.as_ref().unwrap().draft.contains("External"));
    session
        .source
        .as_mut()
        .unwrap()
        .set_draft("Newer unfinished TOML [".into())
        .unwrap();
    let mut durable = retirement.execute();
    assert!(session.busy());
    assert!(other.accept_source_recovery(&mut durable).is_err());
    session.accept_source_recovery(&mut durable).unwrap();
    assert!(!session.busy());
    assert!(session.accept_source_recovery(&mut durable).is_err());
    assert!(session.dirty());
    session.source.as_mut().unwrap().flush_recovery().unwrap();
    session.source.take();
    session.bind(&directory.path().join("schema.toml")).unwrap();
    assert_eq!(
        session.source.as_ref().unwrap().draft,
        "Newer unfinished TOML ["
    );
}

#[test]
fn dispatch_failure_releases_activity_and_source_work_blocks_schema_replacement() {
    let (_directory, mut session) = session().unwrap();
    let work = session.prepare_source(SourceAction::Discard).unwrap();
    assert!(matches!(
        session.check_project_settings("format_version=1\n[project]\nschema='other.json'\n"),
        Err(FileError::SchemaSessionActive)
    ));
    assert!(session.prepare_save().is_err());
    assert!(session.start_generation().is_err());
    let mut receipt = work.not_started("Source worker unavailable".into());
    assert!(session.accept_source(&mut receipt).is_err());
    assert!(!session.busy());
    assert_eq!(session.source.as_ref().unwrap().draft, SOURCE);
}

#[test]
fn recovery_failure_keeps_the_adopted_source_dirty_and_the_archive_discoverable() {
    let (directory, mut session) = session().unwrap();
    let source = session.source.as_mut().unwrap();
    source
        .set_draft("Captured unfinished TOML [".into())
        .unwrap();
    source.flush_recovery().unwrap();
    fs::write(
        directory.path().join("schema.toml"),
        SOURCE.replace("Mara", "External"),
    )
    .unwrap();
    let mut receipt = session
        .prepare_source(SourceAction::Reload)
        .unwrap()
        .execute();
    let archive = receipt.archive().unwrap().to_owned();
    let sidecar = directory
        .path()
        .join("schema.toml.recite-editor-recovery.json");
    fs::remove_file(&sidecar).unwrap();
    fs::create_dir(&sidecar).unwrap();
    let recovery = session.accept_source(&mut receipt).unwrap();
    assert!(session.busy());
    let mut terminal = recovery.execute();
    assert!(session.accept_source_recovery(&mut terminal).is_err());
    assert!(!session.busy());
    assert!(session.dirty());
    let source = session.source.as_ref().unwrap();
    assert!(source.baseline.contains("External"));
    assert_eq!(source.draft, source.baseline);
    assert!(source.recovery_error().is_some());
    assert_eq!(
        fs::read_to_string(archive).unwrap(),
        "Captured unfinished TOML ["
    );
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\nschema='schema.json'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )
    .unwrap();
    let mut project = crate::project::ProjectSession::open(directory.path(), None).unwrap();
    project.parts_mut().0.declarations = Some(session);
    assert!(
        project.checkpoint().is_err(),
        "Close with recovery must check declaration durability."
    );
    assert!(project.dirty());
    fs::remove_dir(sidecar).unwrap();
    project.checkpoint().unwrap();
    assert!(!project.dirty());
}

#[test]
fn editable_artifact_round_trip_preserves_invalid_drafts_and_rejects_foreign_producers() {
    let (directory, mut session) = session().unwrap();
    session
        .source
        .as_mut()
        .unwrap()
        .set_draft("Unfinished TOML [".into())
        .unwrap();
    let work = session.prepare_source(SourceAction::ExportDraft).unwrap();
    assert!(Arc::ptr_eq(&work.schema_output, &session.schema_output));
    let mut receipt = work.execute();
    let artifact = receipt.editable_draft().unwrap().to_owned();
    let recovery = session.accept_source(&mut receipt).unwrap();
    session
        .accept_source_recovery(&mut recovery.execute())
        .unwrap();
    assert_eq!(fs::read_to_string(&artifact).unwrap(), "Unfinished TOML [");
    assert_eq!(session.source.as_ref().unwrap().draft, "Unfinished TOML [");
    fs::write(&artifact, SOURCE.replace("dialogue", "foreign")).unwrap();
    let mut receipt = session
        .prepare_source(SourceAction::ImportDraft(artifact.clone()))
        .unwrap()
        .execute();
    assert!(session.accept_source(&mut receipt).is_err());
    assert!(!session.busy());
    assert_eq!(session.source.as_ref().unwrap().draft, "Unfinished TOML [");
    fs::write(&artifact, SOURCE.replace("Mara", "Repaired")).unwrap();
    let mut receipt = session
        .prepare_source(SourceAction::ImportDraft(artifact.clone()))
        .unwrap()
        .execute();
    let recovery = session.accept_source(&mut receipt).unwrap();
    session
        .accept_source_recovery(&mut recovery.execute())
        .unwrap();
    assert!(session.source.as_ref().unwrap().draft.contains("Repaired"));
    assert_eq!(session.source.as_ref().unwrap().baseline, SOURCE);
    assert_eq!(
        fs::read_to_string(directory.path().join("schema.toml")).unwrap(),
        SOURCE
    );
    assert!(artifact.is_file());
    session.save_and_generate().unwrap();
    assert!(
        fs::read_to_string(&session.output)
            .unwrap()
            .contains("Repaired")
    );
}
#[test]
fn damaged_output_reload_keeps_last_valid_declarations_and_can_regenerate() {
    let (_directory, mut session) = session().unwrap();
    let validated = session.last_valid_schema_snapshot();
    fs::write(&session.output, "unfinished JSON {").unwrap();
    session.reload_source().unwrap();
    assert_eq!(session.output_status(), SchemaOutputStatus::Damaged);
    assert!(Arc::ptr_eq(
        &validated,
        &session.last_valid_schema_snapshot()
    ));
    assert_eq!(session.schema_output.baseline(), "unfinished JSON {");
    assert!(!session.current());
    let mut inspected = session
        .prepare_source(SourceAction::Inspect)
        .unwrap()
        .execute();
    assert_eq!(
        inspected.inspect_source().unwrap(),
        SourceStatus::DamagedOutput
    );
    let recovery = session.accept_source(&mut inspected).unwrap();
    session
        .accept_source_recovery(&mut recovery.execute())
        .unwrap();
    session.save_and_generate().unwrap();
    assert_eq!(session.output_status(), SchemaOutputStatus::Ready);
    assert!(session.current());
    assert_eq!(
        fs::read_to_string(&session.output).unwrap(),
        session.schema_output.baseline()
    );
}

#[test]
fn source_receipt_rejects_a_changed_output_baseline_even_with_the_same_schema() {
    let (_directory, mut session) = session().unwrap();
    let mut receipt = session
        .prepare_source(SourceAction::Discard)
        .unwrap()
        .execute();
    let unchanged_schema = session.last_valid_schema_snapshot();
    session.schema_output = Arc::new(SchemaOutput::Ready(Generated {
        text: format!("{}\n", session.schema_output.baseline()),
        schema: unchanged_schema.clone(),
    }));
    assert!(Arc::ptr_eq(
        &unchanged_schema,
        receipt.schema_output.last_valid_schema()
    ));
    assert!(matches!(
        session.accept_source(&mut receipt),
        Err(FileError::DeclarationSourceChanged)
    ));
    assert!(!session.busy());
}
