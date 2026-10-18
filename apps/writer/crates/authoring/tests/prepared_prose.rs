use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{Document, EditOperation, View, Workbench};

const ID: &str = "11111111111111111111";
const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello, 世界.\n-> END\n\n:: later\n> other@22222222222222222222\n  Later.\n-> END\n";

fn model(source: &str, warm: bool) -> Result<Workbench, Box<dyn std::error::Error>> {
    let document = Document::new(source)?;
    if warm {
        document.script_snapshot()?;
    }
    let mut model = Workbench::from_document(document, Some(View::Source))?;
    model.select(View::Passage(ID.into()))?;
    Ok(model)
}
fn prepare(
    model: &Workbench,
) -> Result<recite_writer_model::PreparedEdit, Box<dyn std::error::Error>> {
    let request = model
        .edit_request(EditOperation::Apply)?
        .ok_or("missing prose request")?;
    std::thread::spawn(move || request.prepare(&CancellationToken::new()))
        .join()
        .map_err(|_| "preparation panicked")?
        .map_err(Into::into)
}

#[test]
fn prepared_prose_preserves_parser_validation_projection_demand_and_history()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [SOURCE.to_owned(), SOURCE.replace('\n', "\r\n")] {
        for warm in [false, true] {
            for text in [
                "Café, 世界!",
                "First line.\nSecond line.",
                ":: syntax",
                " Leading whitespace.",
            ] {
                let mut actual = model(&source, warm)?;
                let mut expected = model(&source, warm)?;
                actual.set_draft(text.into());
                expected.set_draft(text.into());
                let prepared = prepare(&actual);
                let synchronous = expected.apply();
                assert_eq!(prepared.is_ok(), synchronous.is_ok(), "{text}");
                if let Ok(mut prepared) = prepared {
                    actual.accept_edit(&mut prepared)?;
                    assert_eq!(actual.document().source(), expected.document().source());
                    assert_eq!(
                        actual.document().diagnostics(),
                        expected.document().diagnostics()
                    );
                    assert_eq!(actual.draft(), expected.draft());
                    assert!(!actual.has_draft());
                    assert_eq!(actual.document().cached_script().is_some(), warm);
                    assert_eq!(
                        actual.document().passage_snapshot()?,
                        expected.document().passage_snapshot()?
                    );
                    if warm {
                        assert_eq!(
                            actual.document().script_snapshot()?,
                            expected.document().script_snapshot()?
                        );
                    }
                    assert!(actual.accept_edit(&mut prepared).is_err());
                    actual.undo()?;
                    expected.undo()?;
                    assert_eq!(actual.document().source(), expected.document().source());
                    actual.redo()?;
                    expected.redo()?;
                    assert_eq!(actual.document().source(), expected.document().source());
                } else {
                    assert_eq!(actual.document().source(), source);
                    assert_eq!(actual.draft(), text);
                    assert!(actual.has_draft());
                    assert!(!actual.document().can_undo());
                }
            }
        }
    }
    Ok(())
}

#[test]
fn accepted_prose_keeps_newer_typing_including_return_to_previous_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    for newer in ["Hello, 世界.", "Typed while preparing."] {
        let mut model = model(SOURCE, true)?;
        model.set_draft("Submitted.".into());
        let mut prepared = prepare(&model)?;
        model.set_draft(newer.into());
        model.accept_edit(&mut prepared)?;
        assert_eq!(model.draft(), newer);
        assert!(model.has_draft());
        assert!(model.document().source().contains("Submitted."));
        model.discard();
        assert_eq!(model.draft(), "Submitted.");
        model.undo()?;
        assert_eq!(model.draft(), "Hello, 世界.");
    }
    Ok(())
}

#[test]
fn prose_receipt_rejects_changed_analysis_or_selection_without_advancing_history()
-> Result<(), Box<dyn std::error::Error>> {
    for change in 0..3 {
        let mut model = model(SOURCE, true)?;
        model.set_draft("Submitted.".into());
        let mut prepared = prepare(&model)?;
        model.discard();
        match change {
            0 => model.refresh_project(Default::default())?,
            1 => model.select(View::Source)?,
            _ => {
                model.set_draft("Already applied.".into());
                model.apply()?;
            }
        }
        let before = model.document().source_snapshot();
        let history = model.document().history_bytes();
        assert!(model.accept_edit(&mut prepared).is_err());
        assert_eq!(model.document().source(), before.as_ref());
        assert_eq!(model.document().history_bytes(), history);
    }
    let model = model(SOURCE, false)?;
    assert!(model.edit_request(EditOperation::Apply)?.is_none());
    Ok(())
}
