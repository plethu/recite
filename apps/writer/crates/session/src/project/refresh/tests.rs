use super::*;
use crate::project::ProjectSession;
use std::{fs, path::Path};

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Saved prose.\n-> END\n";
fn project() -> (tempfile::TempDir, ProjectSession) {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    fs::write(directory.path().join("a.recite"), SOURCE).unwrap();
    let project = ProjectSession::open(directory.path(), Some(Path::new("a.recite"))).unwrap();
    (directory, project)
}
fn prepare(project: &ProjectSession) -> PreparedRefresh {
    project
        .files()
        .refresh_request(project.current().model())
        .unwrap()
        .prepare(&CancellationToken::new())
        .unwrap()
}

#[test]
fn refresh_accepts_newer_typing_preserves_undo_and_keeps_disk_baselines() {
    let (directory, mut project) = project();
    let (_, model) = project.parts_mut();
    model.set_draft("Applied local prose.".into());
    model.apply().unwrap();
    fs::write(
        directory.path().join("a.recite"),
        SOURCE.replace("Saved", "External"),
    )
    .unwrap();
    fs::write(
        directory.path().join("b.recite"),
        SOURCE.replace("ffee6p8", "rthh7ab"),
    )
    .unwrap();
    let mut prepared = prepare(&project);
    project
        .parts_mut()
        .1
        .set_draft("Newer field typing.".into());
    project.accept_refresh(&mut prepared).unwrap();
    assert_eq!(project.current().model().draft(), "Newer field typing.");
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("Applied local prose")
    );
    assert_eq!(project.documents().len(), 2);
    assert_eq!(
        project
            .files()
            .search_index()
            .search("External prose", 10)
            .0,
        1
    );
    assert!(project.accept_refresh(&mut prepared).is_err());
    let (_, model) = project.parts_mut();
    model.discard();
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    assert!(project.files().dirty("External prose"));
}

#[test]
fn refresh_rejects_newer_accepted_analysis_without_mutating_it() {
    let (_directory, mut project) = project();
    let mut prepared = prepare(&project);
    let (_, model) = project.parts_mut();
    model.set_draft("Accepted after capture.".into());
    model.apply().unwrap();
    assert!(project.accept_refresh(&mut prepared).is_err());
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("Accepted after capture")
    );
}

#[test]
fn refresh_rejects_an_ancestor_manifest_and_excluded_open_documents() {
    let (directory, project) = project();
    let request = project
        .files()
        .refresh_request(project.current().model())
        .unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[discovery]\nexcludes=['a.recite']\n",
    )
    .unwrap();
    fs::write(directory.path().join("other.recite"), SOURCE).unwrap();
    assert!(matches!(
        request.prepare(&CancellationToken::new()),
        Err(FileError::SessionExcluded(_))
    ));
    let child = directory.path().join("child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("recite.project.toml"), "format_version=1\n").unwrap();
    fs::write(child.join("a.recite"), SOURCE).unwrap();
    let child_project = ProjectSession::open(&child, None).unwrap();
    let request = child_project
        .files()
        .refresh_request(child_project.current().model())
        .unwrap();
    fs::remove_file(child.join("recite.project.toml")).unwrap();
    assert!(matches!(
        request.prepare(&CancellationToken::new()),
        Err(FileError::ProjectRootChanged)
    ));
}
