use recite_writer_model::{PreviewSetup, Workbench};
use recite_writer_session::ProjectSession;
use std::fs;

const FIRST: &str =
    ":: first default\n> first@11111111111111111111\n  First.\n-> second.recite::second\n";
const SECOND: &str = ":: second\n> second@22222222222222222222\n  Saved second.\n-> END\n";

fn start(project: &mut ProjectSession) -> Result<(), Box<dyn std::error::Error>> {
    let prepared = project
        .current()
        .model()
        .preview_request()?
        .prepare(PreviewSetup::default())?;
    assert!(project.current_mut().accept_preview(prepared).is_ok());
    Ok(())
}

#[test]
fn retained_preview_reuses_unchanged_inputs_and_only_accepted_sibling_edits_make_it_stale() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )
    .unwrap();
    fs::write(directory.path().join("first.recite"), FIRST).unwrap();
    fs::write(directory.path().join("second.recite"), SECOND).unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    let first = project.current().path().to_owned();
    let second = directory.path().join("second.recite");
    start(&mut project).unwrap();
    project.switch(&second).unwrap();
    project
        .current_mut()
        .edit(|model| {
            model.set_draft("Accepted second.".into());
            Ok(())
        })
        .unwrap();
    project.switch(&first).unwrap();
    assert!(!project.current().model().preview_stale());
    project.current_mut().advance_preview(None).unwrap();
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "Saved second."
    );
    project.switch(&second).unwrap();
    project.current_mut().edit(Workbench::apply).unwrap();
    project
        .current_mut()
        .edit(|model| {
            model.set_draft("Unapplied second.".into());
            Ok(())
        })
        .unwrap();
    project.switch(&first).unwrap();
    assert!(project.current().model().preview_stale());
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "Saved second."
    );
    start(&mut project).unwrap();
    assert!(!project.current().model().preview_stale());
    project.current_mut().advance_preview(None).unwrap();
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "Accepted second."
    );
    assert_eq!(fs::read_to_string(second).unwrap(), SECOND);
    project.checkpoint().unwrap();
    drop(project);
    let project = ProjectSession::open(directory.path(), None).unwrap();
    assert!(
        project.current().model().preview_page().is_none(),
        "trials are transient"
    );
}
