use super::*;
use recite_compiler::authoring::CancellationToken;
use recite_config::{ConfigWriteError, ProjectSettingsError};

fn commit_error(path: &Path) -> ProjectSettingsError {
    ProjectSettingsError::Storage(ConfigWriteError::Commit {
        path: path.to_owned(),
        source: std::io::Error::other("injected commit uncertainty"),
    })
}

#[test]
fn prepared_switches_and_old_refreshes_cannot_cross_manifest_publication() {
    let (directory, mut files, mut model) = fixture();
    let control = CancellationToken::new();
    let target = directory.path().join("second.recite");
    let mut switch = files
        .switch_request(&model, &target, None)
        .unwrap()
        .prepare(&control, |_| Ok(()))
        .unwrap();
    let checkpoint = files
        .begin_navigation_checkpoint(&model)
        .unwrap()
        .wait()
        .unwrap();
    let mut old_refresh = files
        .refresh_request(&model)
        .unwrap()
        .prepare(&control)
        .unwrap();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(
        &mut files,
        &format!("{original}\n[discovery]\nexcludes=['second.recite']\n"),
    );
    let work = files.prepare_settings_publication(&model).unwrap();
    assert!(matches!(
        files.accept_switch(&mut model, &mut switch, &checkpoint),
        Err(FileError::SavePending)
    ));
    assert!(matches!(
        files.switch_request(&model, &target, None),
        Err(FileError::SavePending)
    ));
    assert!(matches!(
        files.begin_navigation_checkpoint(&model),
        Err(FileError::SavePending)
    ));
    files
        .accept_settings_publication(&mut work.execute())
        .unwrap();
    assert!(files.configuration_refresh_required());
    assert!(!files.saving);
    assert!(matches!(
        files.accept_switch(&mut model, &mut switch, &checkpoint),
        Err(FileError::SettingsRefreshRequired)
    ));
    assert!(files.accept_refresh(&mut model, &mut old_refresh).is_err());
    assert!(files.configuration_refresh_required());
    let mut current_refresh = files
        .refresh_request(&model)
        .unwrap()
        .prepare(&control)
        .unwrap();
    files
        .accept_refresh(&mut model, &mut current_refresh)
        .unwrap();
    assert!(!files.configuration_refresh_required());
    assert!(
        files
            .accept_switch(&mut model, &mut switch, &checkpoint)
            .is_err()
    );
    assert!(files.switch_request(&model, &target, None).is_err());
}

#[test]
fn commit_error_readback_reconciles_the_written_prefix_and_reports_the_warning() {
    let (_directory, mut files, model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    let candidate = original.replace("original", "reconciled");
    import(&mut files, &candidate);
    let mut receipt = files
        .prepare_settings_publication(&model)
        .unwrap()
        .execute_with(|owner, source, _| {
            fs::write(owner.path(), source).unwrap();
            Err(commit_error(owner.path()))
        });
    assert!(matches!(
        files.accept_settings_publication(&mut receipt).unwrap(),
        SettingsCommit::PublishedWithWarning(_)
    ));
    assert!(!files.saving);
    assert!(files.configuration_refresh_required());
    assert!(!files.settings.as_ref().unwrap().requires_reload());
    assert_eq!(files.settings.as_ref().unwrap().baseline(), candidate);
    assert_eq!(files.manifest.text(), candidate);
}

#[test]
fn ambiguous_commit_blocks_further_publication_until_explicit_baseline_reload() {
    let (directory, mut files, mut model) = fixture();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(&mut files, &original.replace("original", "candidate"));
    let mut receipt = files
        .prepare_settings_publication(&model)
        .unwrap()
        .execute_with(|owner, _, _| {
            fs::write(owner.path(), "unreadable manifest [").unwrap();
            Err(commit_error(owner.path()))
        });
    assert!(matches!(
        files.accept_settings_publication(&mut receipt).unwrap(),
        SettingsCommit::Unconfirmed(_)
    ));
    assert!(!files.saving);
    assert!(files.settings.as_ref().unwrap().requires_reload());
    assert!(files.configuration_refresh_required());
    fs::write(directory.path().join("recite.project.toml"), &original).unwrap();
    files.refresh(&mut model).unwrap();
    assert!(!files.configuration_refresh_required());
    assert!(matches!(
        files.prepare_settings_publication(&model),
        Err(FileError::SettingsReloadRequired)
    ));
    let mut reload = files
        .prepare_settings(SettingsAction::Reload)
        .unwrap()
        .execute();
    files.accept_settings(&mut reload).unwrap();
    assert!(!files.settings.as_ref().unwrap().requires_reload());
}
