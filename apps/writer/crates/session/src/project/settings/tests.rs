use super::*;
use std::fs;
#[path = "boundary/tests.rs"]
mod boundary;
#[path = "reloading/tests.rs"]
mod reloading;
#[path = "schema/tests.rs"]
mod schema;
fn fixture() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\ncontent_set='original'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("first.recite"),
        ":: first default\n-> END\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("second.recite"),
        ":: second default\n-> END\n",
    )
    .unwrap();
    let mut files = ProjectFiles::open(directory.path()).unwrap();
    let model = files.workbench().unwrap();
    files.settings = Some(Session::open(directory.path()).unwrap());
    (directory, files, model)
}
fn import(files: &mut ProjectFiles, text: &str) -> PathBuf {
    let path = files.root().join("review.toml");
    fs::write(&path, text).unwrap();
    let mut receipt = files
        .prepare_settings(SettingsAction::ImportDraft(path.clone()))
        .unwrap()
        .execute();
    files.accept_settings(&mut receipt).unwrap();
    path
}

#[test]
fn exported_review_keeps_its_original_baseline_and_rejects_later_disk_changes() {
    let (directory, mut files, mut model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    let mut exported = files
        .prepare_settings(SettingsAction::ExportDraft)
        .unwrap()
        .execute();
    let artifact = exported.artifact().unwrap().to_owned();
    assert_eq!(fs::read_to_string(&artifact).unwrap(), original);
    files.accept_settings(&mut exported).unwrap();
    import(&mut files, &original.replace("original", "reviewed"));
    model.set_draft("Unapplied manuscript.".into());
    fs::write(
        directory.path().join("recite.project.toml"),
        original.replace("original", "external"),
    )
    .unwrap();
    let work = files.prepare_settings_publication(&model).unwrap();
    assert!(files.saving);
    assert!(matches!(
        files.require_idle_save(),
        Err(FileError::SavePending)
    ));
    assert!(matches!(
        files
            .accept_settings_publication(&mut work.execute())
            .unwrap(),
        SettingsCommit::Unconfirmed(FileError::SettingsConflict)
    ));
    assert!(files.settings.as_ref().unwrap().requires_reload());
    assert!(!files.saving);
    assert_eq!(files.settings.as_ref().unwrap().baseline(), original);
    assert!(
        fs::read_to_string(directory.path().join("recite.project.toml"))
            .unwrap()
            .contains("external")
    );
    assert_eq!(model.draft(), "Unapplied manuscript.");
    assert!(artifact.is_file());
}

#[test]
fn publication_rechecks_documents_opened_after_import_and_excludes_membership_changes() {
    let (directory, mut files, mut model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(
        &mut files,
        &format!("{original}\n[discovery]\nexcludes=['second.recite']\n"),
    );
    files
        .switch(
            &mut model,
            &directory.path().join("second.recite"),
            None,
            |_| Ok(()),
        )
        .unwrap();
    assert!(matches!(
        files.prepare_settings_publication(&model),
        Err(FileError::ProjectSettings(
            recite_config::ProjectSettingsError::OpenDocument(_)
        ))
    ));
    assert!(!files.saving);
    import(&mut files, &original.replace("original", "reviewed"));
    let work = files.prepare_settings_publication(&model).unwrap();
    assert!(matches!(
        files.switch(
            &mut model,
            &directory.path().join("first.recite"),
            None,
            |_| Ok(())
        ),
        Err(FileError::SavePending)
    ));
    assert!(matches!(
        files.review_rename(&mut model, "first", "renamed"),
        Err(FileError::SavePending)
    ));
    model.set_draft("Typing during the manifest write.".into());
    let mut receipt = work.execute();
    files.settings.as_ref().unwrap().cancel();
    files.accept_settings_publication(&mut receipt).unwrap();
    assert_eq!(model.draft(), "Typing during the manifest write.");
    assert!(
        files
            .settings
            .as_ref()
            .unwrap()
            .baseline()
            .contains("reviewed")
    );
    assert!(!files.manifest.dirty());
    assert!(!files.saving);
    assert!(files.accept_settings_publication(&mut receipt).is_err());
}

#[test]
fn publication_failure_dispatch_and_cancellation_release_the_shared_exclusion() {
    let (_directory, mut files, model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(&mut files, &original.replace("original", "reviewed"));
    let work = files.prepare_settings_publication(&model).unwrap();
    files
        .accept_settings_publication(&mut work.not_started("Worker stopped".into()))
        .unwrap_err();
    assert!(!files.saving);
    let work = files.prepare_settings_publication(&model).unwrap();
    files.settings.as_ref().unwrap().cancel();
    assert!(matches!(
        files.accept_settings_publication(&mut work.execute()),
        Err(FileError::SettingsCancelled)
    ));
    assert!(!files.saving);
    assert_eq!(files.settings.as_ref().unwrap().baseline(), original);
}

#[test]
fn settings_receipts_are_single_owner_and_cancelled_exports_remain_discoverable() {
    let (_first, mut files, _model) = fixture();
    let (_second, mut other, _) = fixture();
    let mut receipt = files
        .prepare_settings(SettingsAction::ExportDraft)
        .unwrap()
        .execute();
    let other_work = other.prepare_settings(SettingsAction::ExportDraft).unwrap();
    assert!(other.accept_settings(&mut receipt).is_err());
    files.settings.as_ref().unwrap().cancel();
    assert!(matches!(
        files.accept_settings(&mut receipt),
        Err(FileError::SettingsCancelled)
    ));
    assert!(receipt.artifact().unwrap().is_file());
    assert!(!files.settings.as_ref().unwrap().busy());
    other.accept_settings(&mut other_work.execute()).unwrap();
}

#[test]
fn foreign_publication_receipts_cannot_release_another_projects_exclusion() {
    let (_first, mut files, model) = fixture();
    let (_second, mut other, other_model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(&mut files, &original.replace("original", "first change"));
    import(&mut other, &original.replace("original", "other change"));
    let work = files.prepare_settings_publication(&model).unwrap();
    let other_work = other.prepare_settings_publication(&other_model).unwrap();
    let mut receipt = work.execute();
    assert!(other.accept_settings_publication(&mut receipt).is_err());
    assert!(other.saving);
    assert!(files.saving);
    files.accept_settings_publication(&mut receipt).unwrap();
    assert!(!files.saving);
    assert!(other.saving);
    other
        .accept_settings_publication(&mut other_work.execute())
        .unwrap();
    assert!(!other.saving);
}
