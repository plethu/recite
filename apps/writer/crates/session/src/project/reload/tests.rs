use super::super::ProjectSession;
use super::*;
use std::fs;

#[path = "cost/tests.rs"]
mod cost;

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Saved prose.\n-> END\n";
fn project() -> Result<(tempfile::TempDir, ProjectSession), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    fs::write(directory.path().join("a.recite"), SOURCE)?;
    fs::write(
        directory.path().join("b.recite"),
        SOURCE
            .replace("start", "other")
            .replace("ffee6p8", "rthh7ab"),
    )?;
    let project = ProjectSession::open(directory.path(), Some(Path::new("a.recite")))?;
    Ok((directory, project))
}
fn prepare(project: &ProjectSession) -> Result<PreparedReload, FileError> {
    project
        .files()
        .reload_request(project.current().model())?
        .prepare(&CancellationToken::new())
}
#[test]
fn reload_prepares_one_disk_snapshot_and_swaps_only_after_archiving_the_complete_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Captured local draft.".into());
        Ok(())
    })?;
    fs::write(
        directory.path().join("a.recite"),
        SOURCE.replace("Saved prose", "Fresh disk prose"),
    )?;
    let mut prepared = prepare(&project)?;
    let archive: Recovery = serde_json::from_str(&fs::read_to_string(prepared.archive())?)?;
    assert_eq!(
        archive.draft.draft_snapshot().as_str(),
        "Captured local draft."
    );
    assert_eq!(project.current().model().draft(), "Captured local draft.");
    project.accept_reload(&mut prepared)?;
    prepared.finish_cleanup()?;
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("Fresh disk prose")
    );
    assert_eq!(
        project
            .files()
            .search_index()
            .search("Fresh disk prose", 10)
            .0,
        1
    );
    assert_eq!(project.documents().len(), 2);
    assert!(!project.current().dirty());
    assert!(project.accept_reload(&mut prepared).is_err());
    Ok(())
}
#[test]
fn newer_typing_rejects_adoption_and_survives_reopening_with_the_archive_kept()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Archived draft.".into());
        Ok(())
    })?;
    let mut prepared = prepare(&project)?;
    project.current_mut().edit(|model| {
        model.set_draft("Newer typing.".into());
        Ok(())
    })?;
    assert!(project.accept_reload(&mut prepared).is_err());
    assert!(prepared.archive().is_file());
    prepared.finish_cleanup()?;
    project.checkpoint()?;
    drop(project);
    let reopened = ProjectSession::open(directory.path(), Some(Path::new("a.recite")))?;
    assert_eq!(reopened.current().model().draft(), "Newer typing.");
    Ok(())
}
#[test]
fn accepted_reload_retains_sibling_drafts_and_history_and_rejects_changed_sibling_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    let other = directory.path().join("b.recite");
    project.switch(&other)?;
    project.current_mut().edit(|model| {
        model.set_draft("Accepted sibling.".into());
        model.apply()?;
        model.set_draft("Sibling field.".into());
        Ok(())
    })?;
    project.switch(&directory.path().join("a.recite"))?;
    let mut prepared = prepare(&project)?;
    project.accept_reload(&mut prepared)?;
    prepared.finish_cleanup()?;
    project.switch(&other)?;
    assert_eq!(project.current().model().draft(), "Sibling field.");
    assert!(project.current().model().document().can_undo());
    project.switch(&directory.path().join("a.recite"))?;
    let mut prepared = prepare(&project)?;
    project
        .parts_mut()
        .0
        .retained
        .get_mut(&other)
        .ok_or("retained sibling")?
        .model
        .set_draft("Newer sibling field.".into());
    assert!(project.accept_reload(&mut prepared).is_err());
    Ok(())
}
#[test]
fn typing_after_adoption_survives_the_cleanup_fence_and_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Old draft.".into());
        Ok(())
    })?;
    let mut prepared = prepare(&project)?;
    project.accept_reload(&mut prepared)?;
    project.current_mut().edit(|model| {
        model.set_draft("After reload.".into());
        Ok(())
    })?;
    prepared.finish_cleanup()?;
    project.checkpoint()?;
    drop(project);
    let reopened = ProjectSession::open(directory.path(), Some(Path::new("a.recite")))?;
    assert_eq!(reopened.current().model().draft(), "After reload.");
    Ok(())
}
#[test]
fn cancelled_preparation_leaves_the_live_draft_and_lease_intact()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Keep this draft.".into());
        Ok(())
    })?;
    let request = project.files().reload_request(project.current().model())?;
    let cancel = CancellationToken::new();
    cancel.interrupt();
    assert!(matches!(
        request.prepare(&cancel),
        Err(FileError::Interrupted(_))
    ));
    assert_eq!(project.current().model().draft(), "Keep this draft.");
    project.checkpoint()?;
    Ok(())
}

#[test]
fn cleanup_failure_reports_after_adoption_and_can_recover_without_rolling_back()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Archived field.".into());
        Ok(())
    })?;
    project.checkpoint()?;
    let sidecar = directory
        .path()
        .join("a.recite.recite-editor-recovery.json");
    fs::remove_file(&sidecar)?;
    fs::create_dir(&sidecar)?;
    fs::write(
        directory.path().join("a.recite"),
        SOURCE.replace("Saved prose", "Reloaded prose"),
    )?;
    let mut prepared = prepare(&project)?;
    project.accept_reload(&mut prepared)?;
    assert!(prepared.finish_cleanup().is_err());
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("Reloaded prose")
    );
    assert!(prepared.archive().is_file());
    assert!(project.current().recovery_error().is_some());
    fs::remove_dir(&sidecar)?;
    // A fresh durability request retries the failed cleanup without replacing accepted source.
    project.checkpoint()?;
    assert!(project.current().recovery_error().is_none());
    assert!(!project.current().dirty());
    Ok(())
}

#[test]
fn a_removed_manifest_cannot_silently_reload_into_the_ancestor_project()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    let nested = directory.path().join("nested");
    fs::create_dir(&nested)?;
    let manifest = nested.join("recite.project.toml");
    fs::write(&manifest, "format_version = 1\n")?;
    fs::write(nested.join("a.recite"), SOURCE)?;
    let mut project = ProjectSession::open(&nested, None)?;
    project.current_mut().edit(|model| {
        model.set_draft("Keep the original project.".into());
        Ok(())
    })?;
    fs::remove_file(manifest)?;
    assert!(matches!(
        prepare(&project),
        Err(FileError::ProjectRootChanged)
    ));
    assert_eq!(
        project.current().model().document().key().as_str(),
        "a.recite"
    );
    assert_eq!(
        project.current().model().draft(),
        "Keep the original project."
    );
    assert!(
        !fs::read_dir(&nested)?.any(|entry| entry.is_ok_and(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(".recite-recovered-")))
    );
    Ok(())
}

#[test]
fn synchronous_reload_cleanup_failure_leaves_the_original_model_open()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut project) = project()?;
    project.current_mut().edit(|model| {
        model.set_draft("Keep until synchronous reload succeeds.".into());
        Ok(())
    })?;
    project.checkpoint()?;
    let sidecar = directory
        .path()
        .join("a.recite.recite-editor-recovery.json");
    fs::remove_file(&sidecar)?;
    fs::create_dir(&sidecar)?;
    fs::write(
        directory.path().join("a.recite"),
        SOURCE.replace("Saved prose", "External prose"),
    )?;
    let (files, model) = project.parts_mut();
    assert!(files.reload(model).is_err());
    assert_eq!(project.current().model().document().source(), SOURCE);
    assert_eq!(
        project.current().model().draft(),
        "Keep until synchronous reload succeeds."
    );
    fs::remove_dir(&sidecar)?;
    project.checkpoint()?;
    let mut prepared = prepare(&project)?;
    project.accept_reload(&mut prepared)?;
    prepared.finish_cleanup()?;
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("External prose")
    );
    Ok(())
}
