use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{Document, DraftSnapshot, EditOperation, View, Workbench};

fn source() -> &'static str {
    ":: start default\n> line@11111111111111111111\n  Hello, 世界.\n-> END\n"
}
fn model() -> Workbench {
    Workbench::from_document(
        Document::new(source()).unwrap_or_else(|e| panic!("document: {e}")),
        Some(View::Source),
    )
    .unwrap_or_else(|e| panic!("workbench: {e}"))
}
fn prepare(model: &Workbench, operation: EditOperation) -> recite_writer_model::PreparedEdit {
    let request = model
        .edit_request(operation)
        .unwrap_or_else(|e| panic!("capture: {e}"))
        .unwrap_or_else(|| panic!("missing history"));
    std::thread::spawn(move || request.prepare(&CancellationToken::new()))
        .join()
        .unwrap_or_else(|_| panic!("preparation thread panicked"))
        .unwrap_or_else(|e| panic!("preparation: {e}"))
}
#[test]
fn acceptance_preserves_newer_typing_including_a_return_to_the_old_baseline() {
    for newer in [source().to_owned(), source().replace("Hello", "Newer")] {
        let mut model = model();
        let submitted = source().replace("Hello", "Submitted");
        model.set_draft_snapshot(DraftSnapshot::from_rope(ropey::Rope::from_str(&submitted)));
        let mut prepared = prepare(&model, EditOperation::Apply);
        model.set_draft(newer.clone());
        model.accept_edit(&mut prepared).unwrap();
        assert_eq!(model.document().source(), submitted);
        assert_eq!(model.draft(), newer);
        assert!(model.has_draft());
        assert!(model.accept_edit(&mut prepared).is_err());
        model.discard();
        let mut undo = prepare(&model, EditOperation::Undo);
        model.accept_edit(&mut undo).unwrap();
        assert_eq!(model.document().source(), source());
        assert!(!model.has_draft());
        let mut redo = prepare(&model, EditOperation::Redo);
        model.accept_edit(&mut redo).unwrap();
        assert_eq!(model.draft(), submitted);
        assert!(!model.has_draft());
    }
}
#[test]
fn preparation_accepts_invalid_source_and_history_matches_synchronous_editing() {
    for source in [
        ":: malformed\n? incomplete(\n",
        ":: start default\r\n> line@11111111111111111111\r\n  café!\r\n-> END\r\n",
        "",
    ] {
        let mut actual = model();
        let mut expected = model();
        actual.set_draft(source.into());
        expected.set_draft(source.into());
        let mut prepared = prepare(&actual, EditOperation::Apply);
        actual.accept_edit(&mut prepared).unwrap();
        expected.apply().unwrap();
        assert_eq!(actual.document().source(), expected.document().source());
        assert_eq!(
            actual.document().diagnostics(),
            expected.document().diagnostics()
        );
        let mut undo = prepare(&actual, EditOperation::Undo);
        actual.accept_edit(&mut undo).unwrap();
        expected.undo().unwrap();
        assert_eq!(actual.document().source(), expected.document().source());
        let mut redo = prepare(&actual, EditOperation::Redo);
        actual.accept_edit(&mut redo).unwrap();
        expected.redo().unwrap();
        assert_eq!(actual.document().source(), expected.document().source());
    }
}
#[test]
fn stale_document_context_or_view_cannot_accept_a_receipt() {
    for replacement in 0..3 {
        let mut model = model();
        model.set_draft(source().replace("Hello", "Submitted"));
        let mut prepared = prepare(&model, EditOperation::Apply);
        match replacement {
            0 => {
                model.set_draft(source().replace("Hello", "Accepted first"));
                model.apply().unwrap();
            }
            1 => model.refresh_project(Default::default()).unwrap(),
            _ => {
                model.discard();
                model.show_script().unwrap();
            }
        }
        let before = model.document().source_snapshot();
        assert!(model.accept_edit(&mut prepared).is_err());
        assert_eq!(model.document().source(), before.as_ref());
    }
}
#[test]
fn unchanged_apply_has_no_history() {
    for rope in [false, true] {
        let mut model = model();
        let accepted = model.document().source_snapshot();
        let revision = model.document().revision();
        if rope {
            model.set_draft_snapshot(DraftSnapshot::from_rope(ropey::Rope::from_str(source())));
        }
        let mut prepared = prepare(&model, EditOperation::Apply);
        model.accept_edit(&mut prepared).unwrap();
        std::thread::spawn(move || drop(prepared)).join().unwrap();
        assert!(!model.document().can_undo());
        assert!(!model.has_draft());
        assert_eq!(model.document().revision(), revision);
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &model.document().source_snapshot()
        ));
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &model.draft_snapshot().text_snapshot()
        ));
        assert!(model.edit_request(EditOperation::Undo).unwrap().is_none());
    }
}

#[test]
fn interrupted_preparation_does_not_publish_text_or_history() {
    let mut model = model();
    model.set_draft(source().replace("Hello", "Submitted"));
    let token = CancellationToken::new();
    token.interrupt();
    assert!(
        model
            .edit_request(EditOperation::Apply)
            .unwrap()
            .unwrap()
            .prepare(&token)
            .is_err()
    );
    assert_eq!(model.document().source(), source());
    assert!(model.has_draft());
    assert!(!model.document().can_undo());
}

#[test]
fn rules_requests_freeze_analysis_and_only_validate_the_matching_rules_draft() {
    let source = ":: start default\n? reply@22222222222222222222 requires=(standing(3))\n  Continue.\n  -> END\n";
    let document = Document::new(source).unwrap();
    let request = document.reply_rules_request("22222222222222222222");
    let mut model = Workbench::from_document(document, Some(View::Source)).unwrap();
    assert!(request.is_current(model.document()));
    let mut rules = std::thread::spawn(move || request.prepare())
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(rules.source().unwrap(), source);
    let request = model.document().reply_rules_request(&rules.passage);
    let mut condition = rules.condition.as_mut().unwrap();
    let arguments = loop {
        use recite_writer_model::RuleExpression;
        match condition {
            RuleExpression::Call { arguments, .. } => break arguments,
            RuleExpression::Group(inner) | RuleExpression::Not(inner) => condition = inner.as_mut(),
            RuleExpression::All(items) | RuleExpression::Any(items) if items.len() == 1 => {
                condition = &mut items[0]
            }
            _ => panic!("expected one condition call"),
        }
    };
    arguments[0].value = "-".into();
    let invalid = rules.source().unwrap();
    model.set_draft(invalid.clone());
    assert!(
        model
            .edit_request(EditOperation::Apply)
            .unwrap()
            .unwrap()
            .with_rules(rules.clone())
            .prepare(&CancellationToken::new())
            .is_err()
    );
    assert_eq!(model.document().source(), source);
    assert_eq!(model.draft(), invalid);
    // Independent Source edits keep their existing permissive diagnostic behavior.
    model.set_draft(source.replace("Continue", "Changed in Source"));
    let mut prepared = model
        .edit_request(EditOperation::Apply)
        .unwrap()
        .unwrap()
        .with_rules(rules)
        .prepare(&CancellationToken::new())
        .unwrap();
    model.accept_edit(&mut prepared).unwrap();
    assert!(!request.is_current(model.document()));
    assert_eq!(request.prepare().unwrap().source().unwrap(), source);
    assert!(model.document().source().contains("Changed in Source"));
}

#[test]
fn queued_rules_requests_do_not_retain_old_passages_and_can_prepare_after_expiry() {
    let source = ":: start default\n? reply@22222222222222222222\n  Continue.\n  -> END\n";
    let document = Document::new(source).unwrap();
    let passages = document.passage_snapshot().unwrap();
    let old = std::sync::Arc::downgrade(&passages);
    let request = document.reply_rules_request("22222222222222222222");
    drop(passages);
    drop(document);
    assert!(old.upgrade().is_none());
    assert_eq!(request.prepare().unwrap().source().unwrap(), source);
}
