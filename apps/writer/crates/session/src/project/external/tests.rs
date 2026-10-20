use super::*;

#[test]
fn adopting_disk_clears_the_draft_and_recovery_and_allows_navigation()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\n",
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        recite_writer_model::FIXTURE,
    )?;
    let mut files = ProjectFiles::open(dir.path())?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    let draft = format!("{}\n# local draft\n", model.document().source());
    model.set_draft(draft.clone());
    files.checkpoint(&model)?;
    let disk = recite_writer_model::FIXTURE.replace("Would you tell me", "Could you tell me");
    std::fs::write(&files.current, &disk)?;
    files.compare_external(&mut model)?;
    let copy = files.resolve_external(&mut model, false)?;

    assert_eq!(model.document().source(), disk);
    assert_eq!(model.draft(), disk);
    assert!(!model.has_draft());
    assert!(!files.dirty(model.document().source()));
    assert!(!files.has_recovery());
    assert!(!files.externally_changed(&files.current));
    assert_eq!(std::fs::read_to_string(&files.current)?, disk);
    let exported: crate::recovery::Recovery =
        serde_json::from_str(&std::fs::read_to_string(copy)?)?;
    let mut recovered = recite_writer_model::Workbench::new(recite_writer_model::FIXTURE)?;
    exported.draft.restore(&mut recovered)?;
    assert_eq!(recovered.draft(), draft);
    let passage = model.document().passages()?[0].id.clone();
    model.select(View::Passage(passage))?;
    Ok(())
}
