use super::*;

fn dirty_project() -> Result<(tempfile::TempDir, TestingRunner, String), Box<dyn std::error::Error>>
{
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Source")?;
    let original = std::fs::read_to_string(dir.path().join("a.recite"))?;
    let draft = original.replace("Hello.", "Saved on close.");
    fill(&mut test, "Hello.", &draft)?;
    Ok((dir, test, draft))
}

#[test]
fn save_and_close_conflict_keeps_the_dialog_and_draft_available()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut test, _) = dirty_project()?;
    let path = dir.path().join("a.recite");
    let external = std::fs::read_to_string(&path)?.replace("Hello.", "External saved text.");
    std::fs::write(&path, &external)?;
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.sync_and_update();
    support::click(&mut test, "Save and close")?;
    assert!(has(&test, "changed on disk"));
    assert_eq!(std::fs::read_to_string(path)?, external);
    support::click(&mut test, "Keep editing")?;
    assert!(has(&test, "Saved on close."));
    Ok(())
}
