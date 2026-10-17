use super::*;
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::Document;
use serde_json::{Value, json};
#[path = "parking/tests.rs"]
mod parking;

fn two_scenes()
-> Result<(tempfile::TempDir, ProjectFiles, Workbench, PathBuf), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    std::fs::write(dir.path().join("a.recite"), ":: start default\n-> END\n")?;
    let other = dir.path().join("b.recite");
    std::fs::write(&other, ":: other\n-> END\n")?;
    let mut files = ProjectFiles::open(dir.path())?;
    let current = files.workbench()?;
    Ok((dir, files, current, other))
}

#[test]
fn grouped_inactive_save_preserves_an_unrelated_active_field_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    let reference = dir.path().join("c.recite");
    std::fs::write(&reference, ":: reference\n-> b.recite::other\n")?;
    files.refresh(&mut current)?;
    files.switch(&mut current, &other, |_| Ok(()))?;
    files.review_rename(&mut current, "other", "renamed")?;
    files.apply_rename(&mut current)?;
    files.switch(&mut current, &original, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft("unfinished active source".into());
    let original_source = current.document().source_snapshot();
    files.save_document(&mut current, &other)?;
    files.close_document(&mut current, &other)?;
    assert_eq!(files.current, original);
    assert_eq!(current.draft(), "unfinished active source");
    assert_eq!(current.document().source(), original_source.as_ref());
    assert_eq!(
        std::fs::read_to_string(&original)?,
        original_source.as_ref()
    );
    assert!(std::fs::read_to_string(&reference)?.contains("b.recite::renamed"));
    assert!(!files.manifest.dirty());
    Ok(())
}

#[test]
fn failed_scene_save_cleanup_blocks_leaving_until_retry() -> Result<(), Box<dyn std::error::Error>>
{
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    let changed = ":: changed\n> line@11111111111111111111\n  Changed text.\n-> END\n";
    current.set_draft(changed.into());
    files.switch(&mut current, &original, |_| Ok(()))?;
    let recovery = other.with_file_name("b.recite.recite-editor-recovery.json");
    std::fs::remove_file(&recovery)?;
    std::fs::create_dir(&recovery)?;
    assert!(matches!(
        files.save_document(&mut current, &other),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert_eq!(std::fs::read_to_string(&other)?, changed);
    assert!(files.retained_dirty());
    assert!(!files.can_leave(&current));
    assert!(
        files
            .open_documents(&current)
            .iter()
            .any(|(path, dirty)| path == &other && *dirty)
    );
    std::fs::remove_dir(&recovery)?;
    files.close_document(&mut current, &other)?;
    files.save_current(&mut current)?;
    assert!(!files.retained_dirty());
    assert!(files.can_leave(&current));
    assert!(!recovery.exists());
    assert!(
        current
            .document()
            .project_sections()
            .contains(&("b.recite".into(), vec!["changed".into()]))
    );
    assert_eq!(files.search_index().search("Changed", 10).0, 1);
    Ok(())
}

#[test]
fn save_all_refreshes_navigation_after_applying_retained_source_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(":: renamed\n-> END\n".into());
    files.switch(&mut current, &original, |_| Ok(()))?;
    files.save_all(&mut current)?;
    assert_eq!(
        current.document().project_sections(),
        vec![
            ("a.recite".into(), vec!["start".into()]),
            ("b.recite".into(), vec!["renamed".into()])
        ]
    );
    Ok(())
}

#[test]
fn failed_save_all_projects_applied_but_unsaved_retained_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    let later = dir.path().join("c.recite");
    std::fs::write(&later, ":: later\n-> END\n")?;
    files.refresh(&mut current)?;
    files.switch(&mut current, &other, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(":: renamed\n-> END\n".into());
    files.switch(&mut current, &later, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(":: changed_later\n-> END\n".into());
    files.switch(&mut current, &original, |_| Ok(()))?;
    std::fs::write(&later, ":: external\n-> END\n")?;
    assert!(matches!(
        files.save_all(&mut current),
        Err(FileError::Conflict)
    ));
    assert_eq!(files.current, original);
    assert_eq!(
        current.document().project_sections(),
        vec![
            ("a.recite".into(), vec!["start".into()]),
            ("b.recite".into(), vec!["renamed".into()]),
            ("c.recite".into(), vec!["changed_later".into()])
        ]
    );
    assert!(files.retained_dirty());
    assert_eq!(std::fs::read_to_string(&other)?, ":: renamed\n-> END\n");
    assert_eq!(std::fs::read_to_string(&later)?, ":: external\n-> END\n");
    Ok(())
}

#[test]
fn inactive_save_conflict_preserves_active_draft_and_both_sessions()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(":: changed\n-> END\n".into());
    files.switch(&mut current, &original, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft("active unfinished draft".into());
    std::fs::write(&other, ":: external\n-> END\n")?;
    assert!(matches!(
        files.save_document(&mut current, &other),
        Err(FileError::Conflict)
    ));
    assert_eq!(files.current, original);
    assert_eq!(current.draft(), "active unfinished draft");
    assert_eq!(current.view(), &recite_writer_model::View::Source);
    assert_eq!(files.open_documents(&current).len(), 2);
    assert!(
        files.retained[&other]
            .model
            .document()
            .source()
            .contains("changed")
    );
    Ok(())
}

#[test]
fn inactive_save_and_close_releases_ownership_without_disturbing_the_active_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(":: changed\n-> END\n".into());
    files.switch(&mut current, &original, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft("active unfinished draft".into());
    files.save_document(&mut current, &other)?;
    files.close_document(&mut current, &other)?;
    assert_eq!(files.current, original);
    assert_eq!(current.draft(), "active unfinished draft");
    assert!(!files.retained.contains_key(&other));
    drop(RecoveryStore::open(&other)?);
    std::fs::write(&other, ":: external\n-> END\n")?;
    files.save_retained()?;
    files.switch(&mut current, &other, |_| Ok(()))?;
    assert_eq!(current.document().source(), ":: external\n-> END\n");
    Ok(())
}

#[test]
fn active_close_cleanup_failure_preserves_the_model_and_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    files.switch(&mut current, &original, |_| Ok(()))?;
    let view = current.view().clone();
    let source = current.document().source_snapshot();
    files.recovery.persist(Some(crate::recovery::Recovery::new(
        files.saved.clone(),
        current.recovery(),
    )))?;
    let recovery = original.with_file_name("a.recite.recite-editor-recovery.json");
    std::fs::remove_file(&recovery)?;
    std::fs::create_dir(&recovery)?;
    assert!(matches!(
        files.close_document(&mut current, &original),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert_eq!(files.current, original);
    assert_eq!(current.document().source(), source.as_ref());
    assert_eq!(current.view(), &view);
    assert_eq!(files.open_documents(&current).len(), 2);
    std::fs::remove_dir(&recovery)?;
    files.close_document(&mut current, &original)?;
    assert_eq!(files.current, other);
    drop(RecoveryStore::open(&original)?);
    Ok(())
}

#[test]
fn close_cleanup_failure_retains_the_session_and_lease_until_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    files.switch(&mut current, &other, |_| Ok(()))?;
    files.switch(&mut current, &original, |_| Ok(()))?;
    let session = files.retained.get_mut(&other).ok_or("retained scene")?;
    session
        .recovery
        .persist(Some(crate::recovery::Recovery::new(
            session.baseline.clone(),
            session.model.recovery(),
        )))?;
    let recovery = other.with_file_name("b.recite.recite-editor-recovery.json");
    std::fs::remove_file(&recovery)?;
    std::fs::create_dir(&recovery)?;
    assert!(matches!(
        files.close_document(&mut current, &other),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert!(files.retained.contains_key(&other));
    assert!(matches!(
        RecoveryStore::open(&other),
        Err(FileError::RecoveryInUse)
    ));
    std::fs::remove_dir(&recovery)?;
    files.close_document(&mut current, &other)?;
    drop(RecoveryStore::open(&other)?);
    Ok(())
}

#[test]
fn excluded_retained_scene_keeps_its_draft_navigation_and_save_path()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    let manifest = root.join("recite.project.toml");
    let original_manifest = "format_version = 1\n[project]\n";
    std::fs::write(&manifest, original_manifest)?;
    let a = root.join("a.recite");
    let b = root.join("b.recite");
    let source = ":: start default\n> line@11111111111111111111\n  Original.\n-> END\n";
    let changed = source.replace("Original.", "Unsaved.");
    std::fs::write(&a, source)?;
    std::fs::write(&b, ":: other\n-> END\n")?;
    let mut files = ProjectFiles::open(&root)?;
    let mut current = files.workbench()?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(changed.clone());
    files.switch(&mut current, &b, |_| Ok(()))?;
    let tabs = files.open_documents(&current);
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\n[discovery]\nexcludes = ['a.recite']\n",
    )?;

    assert!(matches!(
        files.refresh(&mut current),
        Err(FileError::SessionExcluded(path)) if path == a
    ));
    assert_eq!(files.current, b);
    assert_eq!(files.open_documents(&current), tabs);
    assert_eq!(files.manifest.text(), original_manifest);
    assert_eq!(std::fs::read_to_string(&a)?, source);
    files.switch(&mut current, &a, |_| Ok(()))?;
    assert!(current.has_draft());
    assert_eq!(current.draft(), changed);
    files.switch(&mut current, &b, |_| Ok(()))?;
    files.save_retained()?;
    assert_eq!(std::fs::read_to_string(&a)?, changed);
    assert!(!files.retained_dirty());

    std::fs::write(&manifest, original_manifest)?;
    files.refresh(&mut current)?;
    files.switch(&mut current, &a, |_| Ok(()))?;
    assert_eq!(current.document().source(), changed);
    assert!(!current.has_draft());
    Ok(())
}

#[test]
fn extraction_uses_retained_edits_and_refreshes_clean_scenes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    std::fs::write(
        root.join("recite.project.toml"),
        "format_version = 1\n[project]\n",
    )?;
    let a = root.join("a.recite");
    let b = root.join("b.recite");
    let source = ":: start default\n> line@11111111111111111111\n  Original.\n-> END\n";
    std::fs::write(&a, source)?;
    std::fs::write(&b, ":: other\n-> END\n")?;
    let mut files = ProjectFiles::open(&root)?;
    let mut current = files.workbench()?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(source.replace("Original.", "Unsaved."));
    current.apply()?;
    files.switch(&mut current, &b, |_| Ok(()))?;
    let extract =
        |files: &ProjectFiles, current: &Workbench| -> Result<String, Box<dyn std::error::Error>> {
            let discovered = recite_config::discover_project(&root)?;
            let context = files.catalogue_context(crate::project_context::load(&discovered)?)?;
            let document = Document::in_project(
                current.document().key().clone(),
                current.document().source(),
                context,
            )?;
            Ok(document
                .extract_catalogue()
                .catalog
                .ok_or("catalogue")?
                .to_pot_string())
        };
    assert!(extract(&files, &current)?.contains("Unsaved."));
    files.save_retained()?;
    std::fs::write(&a, source.replace("Original.", "External."))?;
    assert!(extract(&files, &current)?.contains("External."));
    files.switch(&mut current, &a, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft("Unapplied".into());
    files.switch(&mut current, &b, |_| Ok(()))?;
    assert!(extract(&files, &current).is_err());
    Ok(())
}

fn measure<T>(samples: &mut Vec<Value>, name: &str, operation: impl FnOnce() -> T) -> T {
    let start = WallTime.start();
    let value = operation();
    samples.push(json!({"operation":name,"milliseconds":WallTime.end(start).as_secs_f64()*1000.}));
    value
}

fn resources() -> Value {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let mut result = serde_json::Map::new();
    for line in status.lines() {
        if let Some((key, value)) = line.split_once(':')
            && matches!(key, "VmRSS" | "VmHWM" | "Threads")
        {
            result.insert(key.into(), json!(value.trim()));
        }
    }
    result.into()
}

#[test]
#[ignore = "explicit retained-session timing and memory workload"]
fn retained_project_sessions_preserve_edits_and_release_closed_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let size = |key: &str, default: usize| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(std::env::var(key)
            .ok()
            .map(|v| v.parse())
            .transpose()?
            .unwrap_or(default))
    };
    let passages = size("RECITE_BENCH_PASSAGES", 10_000)?;
    let visits = size("RECITE_BENCH_VISITS", 20)?;
    let inputs = recite_writer_model::workload::project(passages, 500)?;
    assert!(visits > 1 && visits <= inputs.len());
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    for document in &inputs {
        std::fs::write(dir.path().join(document.key().as_str()), document.text())?;
    }
    let mut samples = Vec::new();
    let initial = resources();
    let mut files = measure(&mut samples, "discovery_and_index", || {
        ProjectFiles::open(dir.path())
    })?;
    let mut current = measure(&mut samples, "initial_workbench", || files.workbench())?;
    assert!(current.document().diagnostics().is_empty());
    let opened = resources();
    let paths = files.paths[..visits].to_vec();
    for (index, path) in paths.iter().enumerate() {
        measure(&mut samples, "first_visit", || {
            files.switch(&mut current, path, |_| Ok(()))
        })?;
        current.set_draft(format!("{} Visit {index}.", current.draft()));
        measure(&mut samples, "apply", || current.apply())?;
    }
    let visited = resources();
    let history_bytes = current.document().history_bytes()
        + files
            .retained
            .values()
            .map(|session| session.model.document().history_bytes())
            .sum::<usize>();
    for (index, path) in paths.iter().enumerate() {
        measure(&mut samples, "revisit", || {
            files.switch(&mut current, path, |_| Ok(()))
        })?;
        assert!(current.draft().ends_with(&format!("Visit {index}.")));
        measure(
            &mut samples,
            "undo_redo",
            || -> Result<(), recite_writer_model::WorkbenchError> {
                current.undo()?;
                current.redo()?;
                Ok(())
            },
        )?;
        assert!(current.draft().ends_with(&format!("Visit {index}.")));
    }
    let revisited = resources();
    measure(&mut samples, "save_all", || files.save_all(&mut current))?;
    let active = files.current.clone();
    for path in paths.iter().filter(|path| *path != &active) {
        measure(&mut samples, "close", || {
            files.close_document(&mut current, path)
        })?;
    }
    assert!(files.retained.is_empty());
    assert!(files.can_leave(&current));
    let closed = resources();
    // This opt-in workload runs alone; retained sessions may own locks and
    // snapshots, but must not each retain an idle recovery thread.
    if let Some(opened_threads) = opened["Threads"].as_str() {
        let budget = opened_threads.parse::<usize>()? + 1;
        for checkpoint in [&visited, &revisited, &closed] {
            let threads = checkpoint["Threads"]
                .as_str()
                .ok_or("thread checkpoint")?
                .parse::<usize>()?;
            assert!(threads <= budget, "{threads} threads exceed {budget}");
        }
    }
    println!(
        "{}",
        json!({"passages":passages,"documents":inputs.len(),"visits":visits,
        "source_bytes":inputs.iter().map(|d| d.text().len()).sum::<usize>(),"history_bytes":history_bytes,
        "resources":{"initial":initial,"opened":opened,"visited":visited,"revisited":revisited,"closed":closed},
        "samples":samples,"host":std::env::consts::OS,"profile":if cfg!(debug_assertions) { "test" } else { "release" },"native_frame_pacing":false})
    );
    Ok(())
}
