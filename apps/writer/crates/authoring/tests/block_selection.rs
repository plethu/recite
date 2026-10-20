use recite_writer_model::{Document, EditError, View, Workbench};

const DUPLICATE: &str = ":: same default\n> one@11111111111111111111\n  First.\n-> END\n:: same\n> two@22222222222222222222\n  Second.\n-> END\n";

#[test]
fn ambiguous_beat_selection_and_structural_edits_preserve_source_revision_and_history() {
    let mut document = Document::new(DUPLICATE).unwrap();
    let revision = document.revision();
    assert!(
        document.script_snapshot().is_ok(),
        "the readable projection does not resolve duplicate names"
    );
    assert!(matches!(
        document.add_line(revision, "same"),
        Err(EditError::SourceRequired(_))
    ));
    assert!(matches!(
        document.add_choice(revision, "same"),
        Err(EditError::SourceRequired(_))
    ));
    assert!(matches!(
        document.set_continuation(revision, "same", "END"),
        Err(EditError::SourceRequired(_))
    ));
    assert!(matches!(
        document.rename_beat(revision, "same", "renamed"),
        Err(EditError::SourceRequired(_))
    ));
    assert_eq!(document.source(), DUPLICATE);
    assert_eq!(document.revision(), revision);
    assert!(!document.undo().unwrap());
    let mut model = Workbench::new(DUPLICATE).unwrap();
    let selected = model.view().clone();
    assert!(model.inspect_block("same").is_err());
    assert_eq!(model.view(), &selected);
    model.set_draft("Keep this draft.".into());
    assert!(model.inspect_block("same").is_err());
    assert_eq!(model.draft(), "Keep this draft.");
}

#[test]
fn recognizable_headers_stay_available_but_failed_projection_does_not_change_selection() {
    let source = ":: start default\n->\n";
    let mut model = Workbench::new(source).unwrap();
    assert_eq!(model.document().block_summaries()[0].id().as_str(), "start");
    assert_eq!(model.view(), &View::Source);
    assert!(model.inspect_block("start").is_err());
    assert_eq!(model.view(), &View::Source);
    assert_eq!(model.document().source(), source);
    assert!(model.inspect_block("missing").is_err());
}

#[test]
fn recovery_can_restore_an_unprojectable_block_without_authorizing_a_structural_edit() {
    let source = ":: start default\n->\n";
    let model = Workbench::from_document(
        Document::new(source).unwrap(),
        Some(View::Block("start".into())),
    )
    .unwrap();
    let recovered = model.recovery();
    let mut reopened = Workbench::new(source).unwrap();
    recovered.restore(&mut reopened).unwrap();
    assert_eq!(reopened.document().source(), source);
    assert_eq!(reopened.view(), &View::Block("start".into()));
    assert!(reopened.inspect_block("start").is_err());
    assert!(reopened.show_script().is_err());
    assert!(reopened.add_line().is_err());
    assert!(reopened.rename_beat("renamed").is_err());
    assert_eq!(reopened.document().source(), source);
}

#[test]
fn history_restores_unprojectable_source_and_keeps_redo_after_rejected_inspection() {
    let invalid = ":: start default\n->\n";
    let valid = ":: start default\n> line@11111111111111111111\n  Restored.\n-> END\n";
    let mut model = Workbench::new(invalid).unwrap();
    model.set_draft(valid.into());
    model.apply().unwrap();
    model
        .select(View::Passage("11111111111111111111".into()))
        .unwrap();
    model.undo().unwrap();
    assert_eq!(model.document().source(), invalid);
    assert!(model.inspect_block("start").is_err());
    assert!(model.show_script().is_err());
    model.redo().unwrap();
    assert_eq!(model.document().source(), valid);
    model.inspect_block("start").unwrap();
}
