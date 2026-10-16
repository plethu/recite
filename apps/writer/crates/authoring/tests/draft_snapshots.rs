use recite_writer_model::{
    DraftSnapshot, EditError, FIXTURE, RecoveredDraft, View, Workbench, WorkbenchError,
};
use ropey::Rope;

#[test]
fn source_selection_and_history_share_accepted_bytes_and_keep_old_snapshots_independent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    let source = model.document().source_snapshot();
    assert_eq!(model.draft().as_ptr(), source.as_ptr());
    let old = model.recovery();
    model.set_draft(format!("{FIXTURE}# Changed\n"));
    model.apply()?;
    assert_eq!(model.draft().as_ptr(), model.document().source().as_ptr());
    assert_eq!(old.draft_snapshot().as_str(), FIXTURE);
    assert_eq!(source.as_ref(), FIXTURE);
    model.undo()?;
    assert_eq!(model.draft().as_ptr(), model.document().source().as_ptr());
    assert_eq!(model.draft(), FIXTURE);
    model.redo()?;
    assert_eq!(model.draft().as_ptr(), model.document().source().as_ptr());
    assert!(model.draft().ends_with("# Changed\n"));
    assert_eq!(old.draft_snapshot().as_str(), FIXTURE);
    Ok(())
}

#[test]
fn dirty_state_tracks_equal_length_edits_and_exact_restoration()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    let original = Rope::from_str(FIXTURE);
    let mut edited = original.clone();
    edited.remove(0..1);
    edited.insert(0, "!");
    assert_eq!(edited.len_bytes(), original.len_bytes());
    model.set_draft_snapshot(DraftSnapshot::from_rope(edited));
    assert!(model.has_draft());
    model.refresh_project(recite_writer_model::ProjectContext::default())?;
    assert!(model.has_draft());
    model.set_draft_snapshot(DraftSnapshot::from_rope(original));
    assert!(!model.has_draft());
    model.set_draft(FIXTURE.replacen(':', "#", 1));
    assert!(model.has_draft());
    model.set_draft(FIXTURE.into());
    assert!(!model.has_draft());
    model.refresh_project(recite_writer_model::ProjectContext::default())?;
    assert!(!model.has_draft());
    Ok(())
}

#[test]
fn source_snapshots_keep_immediate_dirty_state_and_independent_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    let revision = model.document().revision();
    let mut editor = Rope::from_str(FIXTURE);
    model.set_draft_snapshot(DraftSnapshot::from_rope(editor.clone()));
    assert!(!model.has_draft());
    assert!(model.draft_snapshot().matches_rope(&editor));

    editor.insert(editor.len_chars(), "# Unapplied \"é💬\"\\\r\n");
    model.set_draft_snapshot(DraftSnapshot::from_rope(editor.clone()));
    assert!(model.has_draft());
    assert_eq!(model.document().source(), FIXTURE);
    assert_eq!(model.document().revision(), revision);
    assert!(matches!(model.undo(), Err(WorkbenchError::DraftPending)));
    let captured = model.recovery();
    let captured_rope = editor.clone();
    let encoded = serde_json::to_vec(&captured)?;
    let restored_snapshot: RecoveredDraft = serde_json::from_slice(&encoded)?;
    assert_eq!(captured, restored_snapshot);
    assert_eq!(
        serde_json::to_value(&captured)?,
        serde_json::json!({
            "source": FIXTURE,
            "view": "Source",
            "draft": String::from(&captured_rope),
        })
    );

    editor.insert(0, "# Later revision\n");
    model.set_draft_snapshot(DraftSnapshot::from_rope(editor));
    assert!(captured.draft_snapshot().matches_rope(&captured_rope));
    assert!(model.draft_snapshot() != captured.draft_snapshot());
    let mut restored = Workbench::new(FIXTURE)?;
    restored_snapshot.restore(&mut restored)?;
    assert_eq!(restored.view(), &View::Source);
    assert!(restored.draft_snapshot().matches_rope(&captured_rope));
    assert!(restored.has_draft());
    restored.discard();
    assert!(!restored.has_draft());
    assert_eq!(restored.draft(), FIXTURE);
    Ok(())
}

#[test]
fn explicit_source_apply_preserves_revision_and_history_rules()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    let before = model.document().revision();
    let text = format!("{FIXTURE}# New source é💬\n");
    let editor = Rope::from_str(&text);
    model.set_draft_snapshot(DraftSnapshot::from_rope(editor.clone()));
    assert_eq!(model.document().revision(), before);
    model.apply()?;
    assert_eq!(model.document().source(), text);
    assert!(model.document().revision() > before);
    assert!(!model.has_draft());
    assert!(model.draft_snapshot().matches_rope(&editor));
    model.undo()?;
    assert_eq!(model.document().source(), FIXTURE);
    assert_eq!(model.draft(), FIXTURE);
    assert!(!model.has_draft());
    model.redo()?;
    assert_eq!(model.document().source(), text);
    assert_eq!(model.draft(), text);
    assert!(!model.has_draft());
    Ok(())
}

#[test]
fn invalid_source_remains_applicable_and_recoverable() -> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    let invalid = ":: bad\n> bad@bad\n  Keep me.\n";
    model.set_draft_snapshot(DraftSnapshot::from_rope(Rope::from_str(invalid)));
    assert!(model.has_draft());
    model.apply()?;
    assert_eq!(model.document().source(), invalid);
    assert!(!model.document().diagnostics().is_empty());
    assert!(!model.has_draft());
    let recovery = model.recovery();
    let mut restored = Workbench::new(recovery.source())?;
    recovery.restore(&mut restored)?;
    assert_eq!(restored.draft(), invalid);
    assert!(!restored.has_draft());
    Ok(())
}

#[test]
fn rejected_prose_keeps_the_snapshot_and_accepted_revision()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(FIXTURE)?;
    let view = model.view().clone();
    let revision = model.document().revision();
    let text = "Hello\n-> END";
    let editor = Rope::from_str(text);
    model.set_draft_snapshot(DraftSnapshot::from_rope(editor.clone()));
    assert!(matches!(
        model.apply(),
        Err(WorkbenchError::Edit(EditError::NotProse))
    ));
    assert_eq!(model.document().revision(), revision);
    assert_eq!(model.document().source(), FIXTURE);
    assert!(model.has_draft());
    assert!(model.draft_snapshot().matches_rope(&editor));
    let recovery = model.recovery();
    let mut restored = Workbench::new(FIXTURE)?;
    recovery.restore(&mut restored)?;
    assert_eq!(restored.view(), &view);
    assert_eq!(restored.draft(), text);
    assert!(restored.has_draft());
    restored.discard();
    assert!(!restored.has_draft());
    Ok(())
}

#[test]
fn stale_recovery_does_not_discard_a_current_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let mut original = Workbench::new(FIXTURE)?;
    original.select(View::Source)?;
    original.set_draft_snapshot(DraftSnapshot::from_rope(Rope::from_str("old draft")));
    let recovery = original.recovery();
    let source = format!("{FIXTURE}# Different accepted source\n");
    let mut current = Workbench::new(&source)?;
    current.set_draft_snapshot(DraftSnapshot::from_rope(Rope::from_str("current draft")));
    let before = current.draft_snapshot().clone();
    let view = current.view().clone();
    let revision = current.document().revision();
    assert!(matches!(
        recovery.restore(&mut current),
        Err(WorkbenchError::Edit(EditError::Stale))
    ));
    assert_eq!(current.draft_snapshot(), &before);
    assert_eq!(current.view(), &view);
    assert_eq!(current.document().source(), source);
    assert_eq!(current.document().revision(), revision);
    Ok(())
}
