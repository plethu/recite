use super::*;

#[test]
fn reload_excludes_membership_and_foreign_receipts_cannot_release_it() {
    let (_directory, mut files, _model) = fixture();
    let (_other, mut other, _) = fixture();
    let work = files.prepare_settings(SettingsAction::Reload).unwrap();
    let mut foreign = other
        .prepare_settings(SettingsAction::Reload)
        .unwrap()
        .execute();
    assert!(files.saving);
    assert!(files.require_navigation_ready().is_err());
    assert!(files.accept_settings(&mut foreign).is_err());
    assert!(files.saving);
    assert!(
        files
            .accept_settings(&mut work.not_started("Worker stopped".into()))
            .is_err()
    );
    assert!(!files.saving);
    assert!(!files.settings.as_ref().unwrap().requires_reload());
    assert!(!files.configuration_refresh_required());
    other.accept_settings(&mut foreign).unwrap();
}
#[test]
fn cancellation_after_observing_changed_bytes_reconciles_the_baseline() {
    let (directory, mut files, _model) = fixture();
    let changed = files
        .settings
        .as_ref()
        .unwrap()
        .baseline()
        .replace("original", "external");
    fs::write(directory.path().join("recite.project.toml"), &changed).unwrap();
    let mut receipt = files
        .prepare_settings(SettingsAction::Reload)
        .unwrap()
        .execute();
    files.settings.as_ref().unwrap().cancel();
    assert_eq!(
        files.accept_settings(&mut receipt).unwrap(),
        SettingsAccepted::ContextChanged
    );
    assert_eq!(files.settings.as_ref().unwrap().baseline(), changed);
    assert_eq!(files.manifest.text(), changed);
    assert!(!files.saving);
    assert!(files.configuration_refresh_required());
}
#[test]
fn failed_observation_quarantines_the_last_known_baseline_but_pre_read_cancel_does_not() {
    let (directory, mut files, _model) = fixture();
    let work = files.prepare_settings(SettingsAction::Reload).unwrap();
    files.settings.as_ref().unwrap().cancel();
    assert!(files.accept_settings(&mut work.execute()).is_err());
    assert!(!files.settings.as_ref().unwrap().requires_reload());
    fs::write(
        directory.path().join("recite.project.toml"),
        "invalid manifest [",
    )
    .unwrap();
    let mut receipt = files
        .prepare_settings(SettingsAction::Reload)
        .unwrap()
        .execute();
    assert!(files.accept_settings(&mut receipt).is_err());
    assert!(files.settings.as_ref().unwrap().requires_reload());
    assert!(
        files
            .settings
            .as_ref()
            .unwrap()
            .baseline()
            .contains("original")
    );
    assert!(files.configuration_refresh_required());
    assert!(!files.saving);
}
#[test]
fn reload_refuses_pending_grouped_manifest_edits() {
    let (_directory, mut files, _model) = fixture();
    let source = files.manifest.text().replace("original", "rename draft");
    files
        .manifest
        .set(
            source.clone(),
            Default::default(),
            vec!["first.recite".into()],
        )
        .unwrap();
    assert!(matches!(
        files.prepare_settings(SettingsAction::Reload),
        Err(FileError::ProjectEditPending)
    ));
    assert_eq!(files.manifest.text(), source);
    assert!(!files.saving);
}
