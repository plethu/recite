use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{Document, EditOperation, PreparedEdit, View, Workbench, WorkbenchError};

const ONE: &str = ":: one default\n> first@11111111111111111111\n  First.\n-> END\n";
const TWO: &str = ":: one default\n> first@11111111111111111111\n  First.\n-> two\n:: two\n> second@22222222222222222222\n  Second.\n-> END\n";

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn model(before: &str, after: &str, view: View) -> Result<Workbench> {
    let mut model = Workbench::from_document(Document::new(before)?, Some(View::Source))?;
    model.set_draft(after.into());
    model.apply()?;
    model.select(view)?;
    Ok(model)
}
fn prepare(model: &Workbench, operation: EditOperation) -> Result<PreparedEdit> {
    let request = model.edit_request(operation)?.ok_or("history entry")?;
    Ok(
        std::thread::spawn(move || request.prepare(&CancellationToken::new()))
            .join()
            .map_err(|_| "preparation thread panicked")??,
    )
}
fn parity(actual: &Workbench, expected: &Workbench) {
    assert_eq!(actual.document().source(), expected.document().source());
    assert_eq!(actual.view(), expected.view());
    assert_eq!(actual.draft(), expected.draft());
    assert_eq!(
        actual.document().diagnostics(),
        expected.document().diagnostics()
    );
    assert_eq!(actual.document().can_undo(), expected.document().can_undo());
    assert!(!actual.has_draft());
}

#[test]
fn prepared_history_matches_selection_fallbacks_and_accepts_malformed_source() -> Result {
    let empty_two = TWO.replace("> second@22222222222222222222\n  Second.\n", "");
    let malformed = ":: one default\n? incomplete(\n";
    for (before, after, view, restored) in [
        (
            ONE,
            TWO,
            View::Passage("11111111111111111111".into()),
            View::Passage("11111111111111111111".into()),
        ),
        (
            empty_two.as_str(),
            TWO,
            View::Passage("22222222222222222222".into()),
            View::Block("two".into()),
        ),
        (
            ONE,
            TWO,
            View::Passage("22222222222222222222".into()),
            View::Block("one".into()),
        ),
        (
            ONE,
            TWO,
            View::Block("two".into()),
            View::Block("one".into()),
        ),
        (
            "",
            ONE,
            View::Passage("11111111111111111111".into()),
            View::Source,
        ),
        (
            malformed,
            ONE,
            View::Passage("11111111111111111111".into()),
            View::Source,
        ),
        (
            malformed,
            ONE,
            View::Block("one".into()),
            View::Block("one".into()),
        ),
    ] {
        let mut actual = model(before, after, view.clone())?;
        let mut expected = model(before, after, view)?;
        let mut undo = prepare(&actual, EditOperation::Undo)?;
        actual.accept_edit(&mut undo)?;
        expected.undo()?;
        parity(&actual, &expected);
        assert_eq!(actual.view(), &restored);
        assert!(actual.accept_edit(&mut undo).is_err());
        if actual.view() != &View::Source && actual.document().diagnostics().is_empty() {
            assert!(actual.document().cached_passages().is_some());
            assert!(actual.document().cached_script().is_some());
        }
        std::thread::spawn(move || drop(undo))
            .join()
            .map_err(|_| "retirement thread panicked")?;
        let mut redo = prepare(&actual, EditOperation::Redo)?;
        actual.accept_edit(&mut redo)?;
        expected.redo()?;
        parity(&actual, &expected);
        actual.show_script()?;
        expected.show_script()?;
        parity(&actual, &expected);
    }
    Ok(())
}

#[test]
fn newer_prose_or_changed_view_or_analysis_cannot_advance_history() -> Result {
    for changed in ["draft", "view", "analysis"] {
        let view = View::Passage("11111111111111111111".into());
        let mut actual = model(ONE, TWO, view.clone())?;
        let mut prepared = prepare(&actual, EditOperation::Undo)?;
        match changed {
            "draft" => actual.set_draft("Newer typing.".into()),
            "view" => actual.select(View::Passage("22222222222222222222".into()))?,
            _ => actual.refresh_project(Default::default())?,
        }
        let accepted = actual.document().source_snapshot();
        let revision = actual.document().revision();
        assert!(actual.accept_edit(&mut prepared).is_err());
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &actual.document().source_snapshot()
        ));
        assert_eq!(actual.document().revision(), revision);
        if changed == "draft" {
            assert_eq!(actual.draft(), "Newer typing.");
            assert!(actual.has_draft());
            actual.discard();
        }
        actual.select(view)?;
        let mut fresh = prepare(&actual, EditOperation::Undo)?;
        actual.accept_edit(&mut fresh)?;
        assert_eq!(actual.document().source(), ONE);
    }
    Ok(())
}

#[test]
fn empty_history_is_cheap_but_still_requires_an_applied_field() -> Result {
    let mut model = Workbench::from_document(Document::new(ONE)?, Some(View::Source))?;
    model.select(View::Passage("11111111111111111111".into()))?;
    assert!(model.document().cached_script().is_none());
    for operation in [EditOperation::Undo, EditOperation::Redo] {
        assert!(model.edit_request(operation)?.is_none());
        assert!(model.document().cached_script().is_none());
        model.set_draft("Unapplied.".into());
        assert!(matches!(
            model.edit_request(operation),
            Err(WorkbenchError::DraftPending)
        ));
        model.discard();
    }
    Ok(())
}
