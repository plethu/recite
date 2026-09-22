use super::ProjectFiles;
use std::fs;

fn project() -> Result<(tempfile::TempDir, ProjectFiles), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    fs::write(
        dir.path().join("scene.recite"),
        recite_writer_model::FIXTURE,
    )?;
    let files = ProjectFiles::open(dir.path())?;
    Ok((dir, files))
}

#[test]
fn save_preserves_source_and_keeps_previous_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let old = files.saved.to_string();
    let changed = old.replace("Would you tell me", "Could you tell me");
    files.save(&changed)?;
    assert_eq!(fs::read_to_string(&files.current)?, changed);
    assert!(!files.dirty(&changed));
    let backups: Vec<_> = fs::read_dir(files.current.parent().ok_or("parent")?)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|p| {
            p.file_name()
                .to_string_lossy()
                .starts_with(".recite-editor-backup-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(backups[0].path())?, old);
    Ok(())
}

#[test]
fn external_changes_are_not_overwritten() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    fs::write(&files.current, "external writer")?;
    assert!(matches!(
        files.save("my draft"),
        Err(super::FileError::Conflict)
    ));
    assert_eq!(fs::read_to_string(&files.current)?, "external writer");
    assert!(files.dirty("my draft"));
    Ok(())
}

#[test]
fn held_save_lock_preserves_the_file_and_retry_succeeds() -> Result<(), Box<dyn std::error::Error>>
{
    let (_dir, mut files) = project()?;
    let old = files.saved.to_string();
    let lock = files.current.with_file_name(format!(
        "{}.recite-editor.lock",
        files.current.file_name().ok_or("name")?.to_string_lossy()
    ));
    let held = fs::File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock)?;
    held.try_lock()?;
    assert!(matches!(
        files.save("changed"),
        Err(super::FileError::Locked(_))
    ));
    assert_eq!(fs::read_to_string(&files.current)?, old);
    drop(held);
    files.save("changed")?;
    assert_eq!(fs::read_to_string(&files.current)?, "changed");
    Ok(())
}

#[cfg(unix)]
#[test]
fn substituted_symlink_is_not_followed() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let outside = dir.path().join("unrelated");
    fs::write(&outside, "untouched")?;
    fs::remove_file(&files.current)?;
    std::os::unix::fs::symlink(&outside, &files.current)?;
    assert!(matches!(
        files.save("changed"),
        Err(super::FileError::FileKind)
    ));
    assert_eq!(fs::read_to_string(outside)?, "untouched");
    Ok(())
}

#[test]
fn restored_session_refuses_to_overwrite_changes_made_while_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let mut workbench = files.workbench()?;
    workbench.set_draft("A recovered question.".into());
    workbench.apply()?;
    workbench.set_draft("Unaccepted field draft.".into());
    files.checkpoint(&workbench)?;
    let current = files.current.clone();
    drop(files);
    fs::write(&current, "external edit")?;
    let mut files = ProjectFiles::open(dir.path())?;
    let restored = files.workbench()?;
    assert!(
        restored
            .document()
            .source()
            .contains("A recovered question.")
    );
    assert_eq!(restored.draft(), "Unaccepted field draft.");
    assert!(matches!(
        files.save(restored.document().source()),
        Err(super::FileError::Conflict)
    ));
    let (copy, loaded) = files.reload(&restored)?;
    assert_eq!(loaded.document().source(), "external edit");
    let preserved: crate::recovery::Recovery = serde_json::from_str(&fs::read_to_string(copy)?)?;
    assert_eq!(preserved.draft, restored.recovery());
    assert_eq!(fs::read_to_string(current)?, "external edit");
    assert!(!files.has_recovery());
    Ok(())
}

#[test]
fn failed_file_switch_keeps_the_current_session_and_its_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let other = dir.path().join("second.recite");
    fs::write(&other, ":: second\n-> END\n")?;
    fs::write(
        dir.path().join("second.recite.recite-editor-recovery.json"),
        "broken",
    )?;
    let original = files.current.clone();
    let mut model = files.workbench()?;
    files.refresh(&mut model)?;
    assert!(files.switch(&mut model, &other, |_| Ok(())).is_err());
    assert_eq!(files.current, original);
    assert_eq!(
        files.workbench()?.document().source(),
        recite_writer_model::FIXTURE
    );
    Ok(())
}

#[test]
fn saved_search_updates_without_rediscovering_scenes() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let before = files.search_index();
    let changed = files
        .saved
        .replace("Would you tell me", "A unique zeppelin");
    files.save(&changed)?;
    assert_eq!(before.search("zeppelin", 10).0, 0);
    assert_eq!(files.search_index().search("zeppelin", 10).0, 1);
    let other = dir.path().join("second.recite");
    fs::write(&other, ":: second\n-> END\n")?;
    let mut workbench = files.workbench()?;
    assert!(
        files.switch(&mut workbench, &other, |_| Ok(())).is_err(),
        "discovery is an explicit refresh"
    );
    files.refresh(&mut workbench)?;
    files.switch(&mut workbench, &other, |_| Ok(()))?;
    assert_eq!(files.search_index().search("zeppelin", 10).0, 1);
    Ok(())
}

#[test]
fn switching_retains_source_drafts_and_applied_undo_history()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let original = files.current.clone();
    let other = dir.path().join("other.recite");
    fs::write(&other, ":: other default\n-> END\n")?;
    let mut model = files.workbench()?;
    files.refresh(&mut model)?;
    model.select(recite_writer_model::View::Source)?;
    let applied = model
        .document()
        .source()
        .replace("Would you tell me", "Could you tell me");
    model.set_draft(applied.clone());
    model.apply()?;
    model.set_draft(format!("{applied}\n# retained draft\n"));
    let draft = model.draft().to_owned();
    files.switch(&mut model, &other, |_| Ok(()))?;
    assert!(files.retained_dirty());
    files.switch(&mut model, &original, |_| Ok(()))?;
    assert_eq!(model.draft(), draft);
    assert_eq!(model.document().source(), applied);
    model.discard();
    model.undo()?;
    assert!(model.document().source().contains("Would you tell me"));
    Ok(())
}

#[test]
fn save_all_refuses_external_changes_in_a_retained_document()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let original = files.current.clone();
    let other = dir.path().join("other.recite");
    fs::write(&other, ":: other default\n-> END\n")?;
    let mut model = files.workbench()?;
    files.refresh(&mut model)?;
    model.select(recite_writer_model::View::Source)?;
    let draft = format!("{}\n# unsaved\n", model.document().source());
    model.set_draft(draft.clone());
    files.switch(&mut model, &other, |_| Ok(()))?;
    fs::write(&original, "# changed outside Recite\n")?;
    assert!(matches!(
        files.save_retained(),
        Err(super::FileError::Conflict)
    ));
    assert_eq!(files.current, other);
    assert!(files.retained_dirty());
    files.switch(&mut model, &original, |_| Ok(()))?;
    assert_eq!(model.document().source(), draft);
    assert_eq!(fs::read_to_string(&original)?, "# changed outside Recite\n");
    Ok(())
}

#[test]
fn tabs_close_only_saved_sessions_and_restore_the_remaining_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let original = files.current.clone();
    let other = dir.path().join("other.recite");
    fs::write(&other, ":: other default\n-> END\n")?;
    let mut model = files.workbench()?;
    files.refresh(&mut model)?;
    model.select(recite_writer_model::View::Source)?;
    model.set_draft(format!("{}\n# unfinished\n", model.document().source()));
    let draft = model.draft().to_owned();
    files.switch(&mut model, &other, |_| Ok(()))?;
    assert_eq!(files.open_documents(&model).len(), 2);
    assert!(matches!(
        files.close_document(&mut model, &original),
        Err(super::FileError::UnsavedDocument)
    ));
    files.close_document(&mut model, &other)?;
    assert_eq!(files.current, original);
    assert_eq!(model.draft(), draft);
    assert_eq!(files.open_documents(&model), vec![(original.clone(), true)]);
    assert_eq!(fs::read_to_string(original)?, recite_writer_model::FIXTURE);
    Ok(())
}

#[test]
fn project_rename_reviews_references_and_undoes_every_file_together()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, files) = project()?;
    drop(files);
    let first = dir.path().join("scene.recite");
    let second = dir.path().join("other.recite");
    let before =
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> old\n\n:: old\n-> END\n";
    let reference = ":: elsewhere\n-> scene.recite::old\n";
    fs::write(&first, before)?;
    fs::write(&second, reference)?;
    let mut files = ProjectFiles::open(dir.path())?;
    let mut model = files.workbench()?;
    files.switch(&mut model, &first, |_| Ok(()))?;
    files.review_rename(&mut model, "old", "new")?;
    assert_eq!(
        files
            .rename_review
            .as_ref()
            .ok_or("review")?
            .plan
            .changes
            .len(),
        2
    );
    files.apply_rename(&mut model)?;
    assert!(model.document().source().contains(":: new"));
    assert!(
        model
            .document()
            .source()
            .contains("line@11111111111111111111")
    );
    assert_eq!(fs::read_to_string(&first)?, before);
    files.switch(&mut model, &second, |_| Ok(()))?;
    assert!(model.document().source().contains("scene.recite::new"));
    assert!(files.rename_history(&mut model, false)?);
    assert_eq!(model.document().source(), reference);
    files.switch(&mut model, &first, |_| Ok(()))?;
    assert_eq!(model.document().source(), before);
    assert!(files.rename_history(&mut model, true)?);
    assert!(model.document().source().contains(":: new"));
    assert_eq!(fs::read_to_string(second)?, reference);
    Ok(())
}

#[test]
fn reviewed_rename_refuses_a_changed_disk_input() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let mut model = files.workbench()?;
    let block = model.document().script()?[0].id.clone();
    files.review_rename(&mut model, &block, "renamed")?;
    let old = model.document().source().to_owned();
    fs::write(dir.path().join("scene.recite"), "# external change\n")?;
    assert!(matches!(
        files.apply_rename(&mut model),
        Err(super::FileError::Conflict)
    ));
    assert_eq!(model.document().source(), old);
    assert!(files.rename_review.is_some());
    Ok(())
}

#[test]
fn external_comparison_returns_a_draft_and_rechecks_the_inspected_version()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let mut model = files.workbench()?;
    model.select(recite_writer_model::View::Source)?;
    model.set_draft(format!("{}\n# my draft\n", model.document().source()));
    let draft = model.draft().to_owned();
    fs::write(&files.current, "# another writer\n")?;
    files.compare_external(&mut model)?;
    fs::write(&files.current, "# changed again\n")?;
    assert!(matches!(
        files.resolve_external(&mut model, true),
        Err(super::FileError::Conflict)
    ));
    assert_eq!(model.draft(), draft);
    files.compare_external(&mut model)?;
    let copy = files.resolve_external(&mut model, true)?;
    assert!(copy.exists());
    assert_eq!(model.draft(), draft);
    assert_eq!(fs::read_to_string(&files.current)?, "# changed again\n");
    model.apply()?;
    files.save(model.document().source())?;
    assert_eq!(fs::read_to_string(&files.current)?, draft);
    Ok(())
}

#[test]
fn renamed_project_reopens_all_affected_unsaved_documents() -> Result<(), Box<dyn std::error::Error>>
{
    let (dir, files) = project()?;
    drop(files);
    fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )?;
    fs::write(
        dir.path().join("other.recite"),
        ":: other\n-> scene.recite::start\n",
    )?;
    let mut files = ProjectFiles::open(dir.path())?;
    let mut model = files.workbench()?;
    files.switch(&mut model, &dir.path().join("scene.recite"), |_| Ok(()))?;
    files.review_rename(&mut model, "start", "renamed")?;
    files.apply_rename(&mut model)?;
    drop(files);
    let mut files = ProjectFiles::open(dir.path())?;
    let model = files.workbench()?;
    assert_eq!(files.open_documents(&model).len(), 2);
    assert!(files.project_edit_pending(&model));
    files.save(model.document().source())?;
    files.save_retained()?;
    assert!(fs::read_to_string(dir.path().join("scene.recite"))?.contains(":: renamed"));
    assert!(fs::read_to_string(dir.path().join("other.recite"))?.contains("scene.recite::renamed"));
    assert!(!dir.path().join(".recite-manifest-draft.json").exists());
    Ok(())
}

#[test]
fn interrupted_project_checkpoint_restores_every_recorded_document()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, files) = project()?;
    drop(files);
    let first = ":: start default\n-> END\n";
    let second = ":: other\n-> scene.recite::start\n";
    fs::write(dir.path().join("scene.recite"), first)?;
    fs::write(dir.path().join("other.recite"), second)?;
    let mut files = ProjectFiles::open(dir.path())?;
    let snapshots = [("scene.recite", first), ("other.recite", second)]
        .into_iter()
        .map(|(name, before)| {
            let model = recite_writer_model::Workbench::new(&before.replace("start", "renamed"))?;
            Ok((
                name.to_owned(),
                crate::recovery::Recovery::new(before.into(), model.recovery()),
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, recite_writer_model::WorkbenchError>>(
        )?;
    let affected = snapshots.keys().cloned().collect();
    files
        .manifest
        .set(files.manifest.text().to_owned(), snapshots, affected)?;
    // No document checkpoint has been written when the project closes here.
    drop(files);
    let mut files = ProjectFiles::open(dir.path())?;
    let mut model = files.workbench()?;
    assert!(model.document().source().contains("renamed"));
    files.switch(&mut model, &dir.path().join("scene.recite"), |_| Ok(()))?;
    assert!(model.document().source().contains(":: renamed"));
    assert_eq!(fs::read_to_string(dir.path().join("scene.recite"))?, first);
    assert_eq!(fs::read_to_string(dir.path().join("other.recite"))?, second);
    Ok(())
}
