use super::*;
#[test]
fn unavailable_example_reports_its_own_error_without_changing_the_current_editor() {
    let sessions = ExampleSessions::default();
    let mut model = WRITER_EXAMPLES[0].open().unwrap();
    model.set_draft("Keep my pending prose.".into());
    let document = model.document().completion_snapshot();
    let view = model.view().clone();
    assert!(matches!(
        sessions.request(&model, "missing.recite"),
        Err(WorkbenchError::ExampleUnavailable)
    ));
    assert!(document.is_current(model.document()));
    assert_eq!(model.view(), &view);
    assert_eq!(model.draft(), "Keep my pending prose.");
}
#[test]
fn examples_keep_complete_history_and_drafts_without_files() {
    let mut sessions = ExampleSessions::default();
    let mut model = WRITER_EXAMPLES[2].open().unwrap();
    let selected = model.view().clone();
    let original = model.draft().to_owned();
    model.set_draft("Accepted example edit.".into());
    model.apply().unwrap();
    model.set_draft("Retained example draft.".into());
    sessions.switch(&mut model, 0, |_| Ok(())).unwrap();
    sessions.switch(&mut model, 2, |_| Ok(())).unwrap();
    assert_eq!(model.view(), &selected);
    assert_eq!(model.draft(), "Retained example draft.");
    model.discard();
    model.undo().unwrap();
    assert_eq!(model.draft(), original);
    model.redo().unwrap();
    assert_eq!(model.draft(), "Accepted example edit.");
    model.start_preview().unwrap();
    assert!(model.preview_page().is_some());
}
#[test]
fn failed_selection_and_stale_preparation_preserve_both_example_owners() {
    let mut sessions = ExampleSessions::default();
    let mut model = WRITER_EXAMPLES[2].open().unwrap();
    sessions.switch(&mut model, 0, |_| Ok(())).unwrap();
    let original = model.document().key().clone();
    assert!(
        sessions
            .switch(&mut model, 2, |model| model.inspect_block("missing"))
            .is_err()
    );
    assert_eq!(model.document().key(), &original);
    let mut prepared = sessions
        .request(&model, WRITER_EXAMPLES[2].name)
        .unwrap()
        .prepare(&CancellationToken::new(), |_| Ok(()))
        .unwrap();
    model.set_draft("Newer typing.".into());
    assert!(sessions.accept(&mut model, &mut prepared).is_err());
    assert_eq!(model.draft(), "Newer typing.");
    assert!(sessions.switch(&mut model, usize::MAX, |_| Ok(())).is_err());
    sessions.switch(&mut model, 2, |_| Ok(())).unwrap();
    assert_eq!(model.document().key().as_str(), WRITER_EXAMPLES[2].name);
}
