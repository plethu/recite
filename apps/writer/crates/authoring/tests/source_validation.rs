use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{
    Document, EditOperation, RecoveredDraft, SourceValidation, View, Workbench,
};

const SOURCE: &str = ":: start default\n? @rthh7ab\n  Continue.\n  -> END\n";
const INVALID: &str = ":: start default\n? invalid";

#[test]
fn rules_validation_survives_detachment_serialization_and_synchronous_apply() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.open_reply_rules("rthh7ab").unwrap();
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
    model.set_draft(INVALID.into());
    let bytes = serde_json::to_vec(&model.recovery()).unwrap();
    let recovery: RecoveredDraft = serde_json::from_slice(&bytes).unwrap();
    let mut restored = Workbench::new(SOURCE).unwrap();
    recovery.restore(&mut restored).unwrap();
    assert_eq!(restored.source_validation(), SourceValidation::NoNewErrors);
    assert!(restored.apply().is_err());
    assert_eq!(restored.document().source(), SOURCE);
    assert_eq!(restored.draft(), INVALID);
    assert!(restored.has_draft());
    // No Rules controls are passed to the worker, as with a retained Save All draft.
    let request = restored
        .edit_request(EditOperation::Apply)
        .unwrap()
        .unwrap();
    assert!(request.prepare(&CancellationToken::new()).is_err());
    assert_eq!(restored.document().source(), SOURCE);
    assert_eq!(restored.draft(), INVALID);
}

#[test]
fn policy_changes_invalidate_queued_permissive_edits_and_navigation() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.select(View::Source).unwrap();
    let request = model.edit_request(EditOperation::Apply).unwrap().unwrap();
    let navigation = model.navigation_request();
    let mut prepared = request.prepare(&CancellationToken::new()).unwrap();
    model.open_reply_rules("rthh7ab").unwrap();
    assert!(!navigation.is_current(&model));
    assert!(model.accept_edit(&mut prepared).is_err());
    assert_eq!(model.document().source(), SOURCE);
    assert!(!model.has_draft());
}

#[test]
fn prepared_checked_apply_preserves_newer_typing_and_its_validation_policy() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.open_reply_rules("rthh7ab").unwrap();
    let submitted = SOURCE.replace("Continue.", "Submitted.");
    model.set_draft(submitted.clone());
    let request = model.edit_request(EditOperation::Apply).unwrap().unwrap();
    let mut prepared = request.prepare(&CancellationToken::new()).unwrap();
    model.set_draft(INVALID.into());
    model.accept_edit(&mut prepared).unwrap();
    assert_eq!(model.document().source(), submitted);
    assert_eq!(model.draft(), INVALID);
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
    assert!(model.apply().is_err());
    model.discard();
    let request = model.edit_request(EditOperation::Undo).unwrap().unwrap();
    let mut prepared = request.prepare(&CancellationToken::new()).unwrap();
    model.accept_edit(&mut prepared).unwrap();
    assert_eq!(model.document().source(), SOURCE);
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
}

#[test]
fn legacy_recovery_remains_freeform_and_explicit_repair_requires_valid_source() {
    let legacy = serde_json::json!({ "source": SOURCE, "view": "Source", "draft": INVALID });
    let recovery: RecoveredDraft = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(&recovery).unwrap(), legacy);
    let mut freeform = Workbench::new(SOURCE).unwrap();
    recovery.restore(&mut freeform).unwrap();
    assert_eq!(freeform.source_validation(), SourceValidation::Freeform);
    freeform.apply().unwrap();
    assert_eq!(freeform.document().source(), INVALID);

    let mut repair = Workbench::new(SOURCE).unwrap();
    recovery.restore(&mut repair).unwrap();
    repair
        .stage_repaired_source(INVALID.to_owned().into())
        .unwrap();
    assert!(repair.apply().is_err());
    let repaired = SOURCE.replace("Continue.", "Repaired.");
    repair
        .stage_repaired_source(repaired.clone().into())
        .unwrap();
    repair.apply().unwrap();
    assert_eq!(repair.document().source(), repaired);
    assert_eq!(repair.source_validation(), SourceValidation::NoNewErrors);
}

#[test]
fn malformed_recovery_policy_is_rejected_before_current_work_changes() {
    let recovery: RecoveredDraft = serde_json::from_value(serde_json::json!({
        "source": SOURCE,
        "view": { "Passage": "rthh7ab" },
        "draft": "unsafe policy combination",
        "source_validation": "NoNewErrors",
    }))
    .unwrap();
    let mut model = Workbench::new(SOURCE).unwrap();
    model.set_draft("Current unsaved prose.".into());
    let before = model.recovery();
    assert!(recovery.restore(&mut model).is_err());
    assert_eq!(model.recovery(), before);
}

#[test]
fn checked_changes_allow_existing_project_errors_and_refuse_new_ones() {
    let existing = SOURCE.replace("-> END", "-> missing");
    let mut model =
        Workbench::from_document(Document::new(&existing).unwrap(), Some(View::Source)).unwrap();
    model
        .stage_repaired_source(existing.replace("Continue.", "Still valid prose.").into())
        .unwrap();
    model.apply().unwrap();
    let accepted = model.document().source_snapshot();
    model
        .stage_repaired_source(accepted.replace("missing", "another_missing").into())
        .unwrap();
    assert!(model.apply().is_err());
    assert_eq!(model.document().source(), accepted.as_ref());
}

#[test]
fn prepared_validation_reuses_the_candidate_without_accepting_new_project_errors() {
    let existing = SOURCE.replace("-> END", "-> missing");
    let mut model = Workbench::open("scene.recite", &existing).unwrap();
    model.select(View::Source).unwrap();
    model
        .stage_repaired_source(existing.replace("Continue.", "Edited.").into())
        .unwrap();
    let mut prepared = model
        .edit_request(EditOperation::Apply)
        .unwrap()
        .unwrap()
        .prepare(&CancellationToken::new())
        .unwrap();
    model.accept_edit(&mut prepared).unwrap();
    let accepted = model.document().source_snapshot();
    let changed = accepted.replace("missing", "another_missing");
    model.stage_repaired_source(changed.clone().into()).unwrap();
    assert!(
        model
            .edit_request(EditOperation::Apply)
            .unwrap()
            .unwrap()
            .prepare(&CancellationToken::new())
            .is_err()
    );
    assert_eq!(model.document().source(), accepted.as_ref());
    assert_eq!(model.draft(), changed);
}

#[test]
fn invalid_recovery_selections_preserve_the_current_field_and_its_policy() {
    for view in [
        View::Passage("missing".into()),
        View::Block("missing".into()),
    ] {
        let recovery: RecoveredDraft = serde_json::from_value(serde_json::json!({
            "source": SOURCE, "view": view, "draft": "rejected recovery",
        }))
        .unwrap();
        let mut model = Workbench::new(SOURCE).unwrap();
        model.open_reply_rules("rthh7ab").unwrap();
        model.set_draft("Current unsaved rules.".into());
        let before = model.recovery();
        assert!(recovery.restore(&mut model).is_err());
        assert_eq!(model.recovery(), before);
    }
}
