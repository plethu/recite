use recite_writer_model::{View, Workbench};
use recite_writer_session::{project::ProjectFiles, recovery::Recovery};

#[test]
fn repair_exports_the_captured_field_with_backup_and_stays_out_of_source_discovery()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = ":: start default\n> line@11111111111111111111\n  Original.\n-> END\n";
    let path = directory.path().join("scene.recite");
    std::fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    std::fs::write(&path, source)?;
    let mut files = ProjectFiles::open(directory.path())?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    let draft = ":: start default\n? invalid café 💬";
    model.stage_repaired_source(draft.to_owned().into())?;
    let request = files.source_repair_export(&model)?;
    model.set_draft("Newer draft stays open.".into());
    let artifact = std::thread::spawn(move || request.write())
        .join()
        .unwrap()?;
    assert_eq!(std::fs::read_to_string(&artifact.source)?, draft);
    assert_eq!(std::fs::read_to_string(&path)?, source);
    assert_eq!(model.draft(), "Newer draft stays open.");
    let backup: Recovery = serde_json::from_str(&std::fs::read_to_string(&artifact.recovery)?)?;
    assert_eq!(backup.draft.source(), source);
    assert_eq!(backup.draft.draft_snapshot().as_str(), draft);
    let report = recite_config::discover_project(directory.path())?;
    assert_eq!(
        report.documents().len(),
        1,
        "repair text and JSON cannot enter compilation"
    );

    // Exported text is a durable user artifact even after the original session exits.
    drop(files);
    assert_eq!(std::fs::read_to_string(&artifact.source)?, draft);
    let mut restored = Workbench::new(source)?;
    backup.draft.restore(&mut restored)?;
    assert_eq!(restored.draft(), draft);
    assert!(restored.apply().is_err());
    Ok(())
}
