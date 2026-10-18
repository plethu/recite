use super::*;
impl ProjectFiles {
    pub(crate) fn save_plan(
        &self,
        current: &mut Workbench,
        scope: Scope,
    ) -> Result<Vec<Step>, FileError> {
        self.require_idle_save()?;
        if matches!(scope, Scope::Declarations) {
            return Ok(vec![Step::Declarations { force: true }]);
        }
        if scope.applies_current(&self.current) {
            current.apply()?;
        }
        self.save_plan_accepted(current, scope)
    }
}
use crate::project::tests::project;
use recite_writer_model::View;

#[test]
fn disk_receipt_preserves_newer_accepted_edits_and_field_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    let requested = model
        .document()
        .source()
        .replace("Would you tell me", "A zeppelin asks");
    model.set_draft(requested.clone());
    let step = files.save_plan(&mut model, Scope::Current)?.remove(0);
    let mut write = files.prepare_write(step)?;
    files.saving = true;
    assert!(matches!(
        files.save_plan(&mut model, Scope::All),
        Err(FileError::SavePending)
    ));
    assert!(matches!(files.reload(&model), Err(FileError::SavePending)));
    assert!(!files.can_leave(&model));
    // An accepted edit and another unaccepted draft arrive before disk publication.
    let newer = requested.replace("A zeppelin asks", "A newer courier asks");
    model.set_draft(newer.clone());
    model.apply()?;
    let unaccepted = newer.replace("A newer courier asks", "An unfinished courier asks");
    model.set_draft(unaccepted.clone());
    let source = model.document().source_snapshot();
    let worker = std::thread::spawn(move || {
        write.execute().unwrap();
        write
    });
    let mut write = worker.join().unwrap();
    let saved = write.source.clone();
    files.publish_write(&model, &mut write)?.wait()?;
    assert!(Arc::ptr_eq(&saved, &files.saved));
    assert!(Arc::ptr_eq(&source, &model.document().source_snapshot()));
    assert_eq!(std::fs::read_to_string(&files.current)?, requested);
    assert_eq!(model.document().source(), newer);
    assert_eq!(model.draft(), unaccepted);
    assert!(model.has_draft());
    assert!(files.dirty(model.document().source()));
    assert_eq!(files.search.search("zeppelin", 100).0, 1);
    files.saving = false;
    drop(files);
    let mut reopened = crate::project::ProjectFiles::open(dir.path())?;
    let recovered = reopened.workbench()?;
    assert_eq!(recovered.document().source(), newer);
    assert_eq!(recovered.draft(), unaccepted);
    assert!(recovered.has_draft());
    Ok(())
}

#[test]
fn cleanup_failure_keeps_saved_baseline_and_retry_clears_the_sidecar()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    model.set_draft(
        model
            .document()
            .source()
            .replace("Would you tell me", "A zeppelin asks"),
    );
    files.checkpoint(&model)?;
    let sidecar = files
        .current
        .with_file_name("scene.recite.recite-editor-recovery.json");
    std::fs::remove_file(&sidecar)?;
    std::fs::create_dir(&sidecar)?;
    let step = files.save_plan(&mut model, Scope::Current)?.remove(0);
    let mut write = files.prepare_write(step)?;
    write.execute()?;
    assert!(matches!(
        files.publish_write(&model, &mut write)?.wait(),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert_eq!(
        std::fs::read_to_string(&files.current)?,
        model.document().source()
    );
    assert!(files.dirty(model.document().source()));
    std::fs::remove_dir(&sidecar)?;
    files.save_current(&mut model)?;
    assert!(!files.dirty(model.document().source()));
    assert!(!sidecar.exists());
    Ok(())
}

#[test]
fn giant_save_preparation_shares_source_and_index_and_rejects_repeat_work()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    let mut source = String::from(":: giant default\n");
    for n in 0..20_000 {
        source.push_str(&format!(
            "> line_{n}@{n:020x}\n  The courier waits beside the café.\n"
        ));
    }
    source.push_str("-> END\n");
    std::fs::write(dir.path().join("scene.recite"), &source)?;
    let mut files = crate::project::ProjectFiles::open(dir.path())?;
    let mut model = files.workbench_with_control(
        Some(View::Source),
        &recite_compiler::authoring::CancellationToken::new(),
    )?;
    let accepted = model.document().source_snapshot();
    let index = files.search.clone();
    for _ in 0..100 {
        let write = files.prepare_write(Step::Current(accepted.clone()))?;
        assert!(Arc::ptr_eq(&write.source, &accepted));
        assert!(Arc::ptr_eq(&write.baseline, &files.saved));
        assert!(Arc::ptr_eq(&write.search, &index));
    }
    files.saving = true;
    for _ in 0..100 {
        assert!(matches!(
            files.save_plan(&mut model, Scope::Current),
            Err(FileError::SavePending)
        ));
    }
    assert_eq!(index.search_results("courier", 100).total(), 20_000);
    assert_eq!(model.document().source(), source);
    Ok(())
}

#[test]
fn saving_a_retained_document_does_not_apply_the_active_invalid_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let active = files.current.clone();
    let saved_active = std::fs::read_to_string(&active)?;
    let retained = dir.path().join("other.recite");
    std::fs::write(&retained, ":: other default\n-> END\n")?;
    let mut model = files.workbench()?;
    files.refresh(&mut model)?;
    files.switch(&mut model, &retained, None, |_| Ok(()))?;
    model.select(View::Source)?;
    let edited = ":: retained default\n-> END\n";
    model.set_draft(edited.into());
    files.switch(&mut model, &active, None, |_| Ok(()))?;
    model.select(View::Source)?;
    model.set_draft(":: unfinished\n? invalid".into());
    let step = files
        .save_plan(&mut model, Scope::Document(retained.clone()))?
        .remove(0);
    let mut write = files.prepare_write(step)?;
    write.execute()?;
    files.publish_write(&model, &mut write)?.wait()?;
    assert_eq!(std::fs::read_to_string(retained)?, edited);
    assert_eq!(std::fs::read_to_string(active)?, saved_active);
    assert_eq!(model.draft(), ":: unfinished\n? invalid");
    assert!(model.has_draft());
    Ok(())
}

#[test]
fn captured_apply_receipt_saves_that_source_without_consuming_newer_typing()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    let submitted = model
        .document()
        .source()
        .replace("Would you tell me", "A submitted courier asks");
    model.set_draft(submitted.clone());
    let request = model
        .edit_request(recite_writer_model::EditOperation::Apply)?
        .ok_or("request")?;
    let mut prepared = std::thread::spawn(move || {
        request.prepare(&recite_compiler::authoring::CancellationToken::new())
    })
    .join()
    .unwrap()?;
    let newer = submitted.replace("A submitted courier asks", "A newer courier asks");
    model.set_draft(newer.clone());
    model.accept_edit(&mut prepared)?;
    files.set_saving(true);
    let step = files.save_plan_accepted(&model, Scope::Current)?.remove(0);
    let mut write = files.prepare_write(step)?;
    assert_eq!(write.source.as_ref(), submitted);
    write.execute()?;
    files.publish_write(&model, &mut write)?.wait()?;
    files.set_saving(false);
    assert_eq!(model.draft(), newer);
    assert!(model.has_draft());
    assert_eq!(std::fs::read_to_string(&files.current)?, submitted);
    Ok(())
}
