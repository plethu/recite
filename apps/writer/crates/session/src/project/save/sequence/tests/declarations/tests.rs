use super::*;
const SOURCE: &str =
    "schema_version = 1\n[producer]\nid = 'dialogue'\n[speakers.mara]\ndisplay_name = 'Mara'\n";

fn declarations() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\nschema = 'schema.json'\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )
    .unwrap();
    fs::write(dir.path().join("schema.toml"), SOURCE).unwrap();
    let schema = recite_core::schema::SchemaSource::load_str("schema.toml", SOURCE)
        .source
        .unwrap();
    fs::write(dir.path().join("schema.json"), schema.export_json()).unwrap();
    let mut files = ProjectFiles::open(dir.path()).unwrap();
    let model = files.workbench().unwrap();
    let mut session = crate::declarations::Session::open(dir.path()).unwrap();
    session.bind(&dir.path().join("schema.toml")).unwrap();
    session.source.as_mut().unwrap().draft = SOURCE.replace("Mara", "Saved name");
    files.declarations = Some(session);
    (dir, files, model)
}

#[test]
fn source_success_then_generated_output_conflict_retains_both_baselines_and_newer_draft() {
    let (dir, mut files, mut model) = declarations();
    let output = dir.path().join("schema.json");
    let mut sequence = SaveSequence::accepted(&mut files, &model, Scope::Declarations);
    loop {
        let Progress::Work(work) = sequence.poll(&mut files, &mut model) else {
            panic!()
        };
        let output_write = matches!(work, Work::DeclarationOutput(_));
        if output_write {
            fs::write(&output, "External generated output").unwrap();
        }
        let receipt = work.execute();
        if matches!(receipt, Receipt::DeclarationSource(..)) {
            files
                .declarations
                .as_mut()
                .unwrap()
                .source
                .as_mut()
                .unwrap()
                .draft = SOURCE.replace("Mara", "Newer unsaved name");
        }
        sequence.accept(&mut files, &mut model, receipt);
        if output_write {
            break;
        }
    }
    assert!(matches!(
        finish(&mut sequence, &mut files, &mut model),
        Err(FileError::Conflict)
    ));
    assert!(!files.saving);
    let source = files
        .declarations
        .as_ref()
        .unwrap()
        .source
        .as_ref()
        .unwrap();
    assert_eq!(source.baseline, SOURCE.replace("Mara", "Saved name"));
    assert!(source.draft.contains("Newer unsaved name"));
    assert_eq!(
        fs::read_to_string(dir.path().join("schema.toml")).unwrap(),
        source.baseline
    );
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "External generated output"
    );
    drop(files);
    // Restore the original generated file to reopen its unchanged producer owner.
    let schema = recite_core::schema::SchemaSource::load_str("schema.toml", SOURCE)
        .source
        .unwrap();
    fs::write(&output, schema.export_json()).unwrap();
    let mut reopened = crate::declarations::Session::open(dir.path()).unwrap();
    reopened.bind(&dir.path().join("schema.toml")).unwrap();
    assert!(
        reopened
            .source
            .as_ref()
            .unwrap()
            .draft
            .contains("Newer unsaved name")
    );
}

#[test]
fn successful_output_refresh_keeps_typing_that_arrived_during_publication() {
    let (_dir, mut files, mut model) = declarations();
    let mut sequence = SaveSequence::accepted(&mut files, &model, Scope::Declarations);
    loop {
        match sequence.poll(&mut files, &mut model) {
            Progress::Work(work) => {
                let receipt = work.execute();
                if matches!(receipt, Receipt::DeclarationOutput(..)) {
                    files
                        .declarations
                        .as_mut()
                        .unwrap()
                        .source
                        .as_mut()
                        .unwrap()
                        .draft = SOURCE.replace("Mara", "Latest typing");
                }
                sequence.accept(&mut files, &mut model, receipt);
            }
            Progress::Finished(result) => {
                result.unwrap();
                break;
            }
            Progress::Waiting => panic!(),
        }
    }
    let session = files.declarations.as_ref().unwrap();
    assert!(session.dirty());
    assert!(
        session
            .source
            .as_ref()
            .unwrap()
            .draft
            .contains("Latest typing")
    );
    assert!(!files.saving);
    assert!(
        session
            .last_valid_schema()
            .speakers
            .values()
            .any(|speaker| speaker.display_name.as_deref() == Some("Saved name"))
    );
}
