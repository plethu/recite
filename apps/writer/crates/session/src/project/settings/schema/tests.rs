use super::*;
use crate::declarations::{Session as Declarations, recovery::Store};
use recite_core::schema::SchemaSource;
const SOURCE: &str =
    "schema_version=1\n[producer]\nid='dialogue'\n[speakers.mara]\ndisplay_name='Mara'\n";

fn with_schema() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let (directory, mut files, mut model) = fixture();
    let manifest = files.settings.as_ref().unwrap().baseline().replace(
        "content_set='original'",
        "content_set='original'\nschema='schema.json'",
    );
    fs::write(directory.path().join("recite.project.toml"), manifest).unwrap();
    let json = SchemaSource::load_str("schema.toml", SOURCE)
        .source
        .unwrap()
        .export_json();
    for file in ["schema.json", "next.json"] {
        fs::write(directory.path().join(file), &json).unwrap();
    }
    fs::write(directory.path().join("schema.toml"), SOURCE).unwrap();
    files.refresh(&mut model).unwrap();
    files.settings = Some(Session::open(directory.path()).unwrap());
    let mut declarations = Declarations::open(directory.path()).unwrap();
    declarations
        .bind(&directory.path().join("schema.toml"))
        .unwrap();
    files.declarations = Some(declarations);
    (directory, files, model)
}

#[test]
fn failed_saved_manifest_observation_retires_clean_producer_authority() {
    let (directory, mut files, _model) = with_schema();
    fs::write(
        directory.path().join("recite.project.toml"),
        "invalid TOML [",
    )
    .unwrap();
    let mut receipt = files
        .prepare_settings(SettingsAction::Reload)
        .unwrap()
        .execute();
    assert!(files.accept_settings(&mut receipt).is_err());
    assert!(files.declarations.is_none());
    assert!(receipt.retired_declarations.is_some());
    assert!(files.settings.as_ref().unwrap().requires_reload());
    assert!(files.configuration_refresh_required());
    assert!(!files.saving);
}

#[test]
fn any_settings_publication_refuses_dirty_declarations_including_the_same_schema() {
    let (_directory, mut files, model) = with_schema();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    files
        .declarations
        .as_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .set_draft(SOURCE.replace("Mara", "Unfinished name"))
        .unwrap();
    import(&mut files, &original.replace("schema.json", "next.json"));
    assert!(matches!(
        files.prepare_settings_publication(&model),
        Err(FileError::SchemaSessionActive)
    ));
    assert!(!files.saving);
    import(&mut files, &original.replace("original", "revised"));
    assert!(matches!(
        files.prepare_settings_publication(&model),
        Err(FileError::SchemaSessionActive)
    ));
    assert!(!files.saving);
    let source = files
        .declarations
        .as_ref()
        .unwrap()
        .source
        .as_ref()
        .unwrap();
    assert!(source.draft.contains("Unfinished name"));
    assert_eq!(source.baseline, SOURCE);
}

#[test]
fn a_declaration_draft_with_failed_recovery_cannot_enter_an_uncertain_manifest_write() {
    let (directory, mut files, model) = with_schema();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    let source = files
        .declarations
        .as_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap();
    source.set_draft("first recovered draft [".into()).unwrap();
    source.flush_recovery().unwrap();
    let recovery = directory
        .path()
        .join("schema.toml.recite-editor-recovery.json");
    fs::remove_file(&recovery).unwrap();
    fs::create_dir(&recovery).unwrap();
    source
        .set_draft("only remaining in-memory draft [".into())
        .unwrap();
    assert!(source.flush_recovery().is_err());
    import(&mut files, &original.replace("original", "candidate"));
    assert!(matches!(
        files.prepare_settings_publication(&model),
        Err(FileError::SchemaSessionActive)
    ));
    assert!(!files.saving);
    assert_eq!(
        files
            .declarations
            .as_ref()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .draft,
        "only remaining in-memory draft ["
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("recite.project.toml")).unwrap(),
        original
    );
}

#[test]
fn successful_schema_change_retires_old_authority_before_a_failed_context_refresh() {
    let (directory, mut files, mut model) = with_schema();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    let candidate = original.replace("schema.json", "next.json");
    import(&mut files, &candidate);
    let work = files.prepare_settings_publication(&model).unwrap();
    model.set_draft("Typing while settings publish.".into());
    let mut receipt = work.execute();
    files.accept_settings_publication(&mut receipt).unwrap();
    assert!(files.declarations.is_none());
    assert!(!files.saving);
    assert_eq!(files.settings.as_ref().unwrap().baseline(), candidate);
    assert_eq!(files.manifest.text(), candidate);
    assert!(Store::open(&directory.path().join("schema.toml")).is_err());
    fs::write(
        directory.path().join("recite.project.toml"),
        "invalid manifest [",
    )
    .unwrap();
    let refresh = files.refresh_request(&model).unwrap();
    assert!(
        refresh
            .prepare(&recite_compiler::authoring::CancellationToken::new())
            .is_err()
    );
    assert_eq!(files.settings.as_ref().unwrap().baseline(), candidate);
    assert_eq!(model.draft(), "Typing while settings publish.");
    drop(receipt);
    assert!(Store::open(&directory.path().join("schema.toml")).is_ok());
}

#[test]
fn unconfirmed_commit_retires_schema_authority_while_verified_prewrite_failure_keeps_it() {
    let (_directory, mut files, model) = with_schema();
    let original = files.settings.as_ref().unwrap().baseline().to_owned();
    import(&mut files, &original.replace("schema.json", "next.json"));
    let error = |path: &Path| {
        recite_config::ProjectSettingsError::Storage(recite_config::ConfigWriteError::Commit {
            path: path.to_owned(),
            source: std::io::Error::other("injected commit failure"),
        })
    };
    let mut unchanged = files
        .prepare_settings_publication(&model)
        .unwrap()
        .execute_with(|owner, _, _| Err(error(owner.path())));
    assert!(files.accept_settings_publication(&mut unchanged).is_err());
    assert!(!files.configuration_refresh_required());
    assert!(files.declarations.is_some());
    let mut unknown = files
        .prepare_settings_publication(&model)
        .unwrap()
        .execute_with(|owner, _, _| {
            fs::write(owner.path(), "unconfirmed manifest [").unwrap();
            Err(error(owner.path()))
        });
    assert!(matches!(
        files.accept_settings_publication(&mut unknown).unwrap(),
        SettingsCommit::Unconfirmed(_)
    ));
    assert!(files.configuration_refresh_required());
    assert!(files.declarations.is_none());
    assert!(files.settings.as_ref().unwrap().requires_reload());
    assert!(!files.saving);
}
