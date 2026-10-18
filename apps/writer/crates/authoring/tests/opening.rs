use recite_writer_model::{Document, EditError, FIXTURE, View, Workbench, WorkbenchError};

#[test]
fn explicit_source_shares_bytes_and_returns_to_the_same_first_script_block()
-> Result<(), WorkbenchError> {
    let document = Document::new(FIXTURE)?;
    let source = document.source_snapshot();
    let mut direct = Workbench::from_document(document, Some(View::Source))?;
    assert_eq!(direct.view(), &View::Source);
    assert_eq!(direct.draft().as_ptr(), source.as_ptr());
    assert!(!direct.has_draft());
    let mut automatic = Workbench::new(FIXTURE)?;
    assert!(matches!(automatic.view(), View::Passage(_)));
    automatic.select(View::Source)?;
    automatic.show_script()?;
    direct.show_script()?;
    assert!(matches!(direct.view(), View::Block(_)));
    assert_eq!(direct.view(), automatic.view());
    assert_eq!(direct.document().script()?, automatic.document().script()?);
    Ok(())
}

#[test]
fn explicit_selection_keeps_validation_and_source_repair_available() -> Result<(), WorkbenchError> {
    assert!(matches!(
        Workbench::from_document(
            Document::new(FIXTURE)?,
            Some(View::Passage("missing".into()))
        ),
        Err(WorkbenchError::Edit(EditError::MissingPassage))
    ));
    for source in ["", "not valid Recite\n"] {
        let mut model = Workbench::from_document(Document::new(source)?, Some(View::Source))?;
        assert_eq!(model.draft(), source);
        assert!(model.show_script().is_err());
        model.set_draft(FIXTURE.into());
        model.apply()?;
        model.show_script()?;
        assert!(matches!(model.view(), View::Block(_)));
    }
    Ok(())
}

#[test]
fn script_fallback_opens_the_first_block_even_when_it_has_only_effects()
-> Result<(), WorkbenchError> {
    let mut model = Workbench::new(FIXTURE)?;
    model.select(View::Source)?;
    model.set_draft(
        concat!(
            ":: effects_first default\n! immediate ring_bell()\n-> later\n",
            ":: later\n> line@11111111111111111111\n  Later prose.\n-> END\n",
        )
        .into(),
    );
    model.apply()?;
    model.show_script()?;
    assert_eq!(model.view(), &View::Block("effects_first".into()));
    Ok(())
}

#[test]
fn source_outline_changes_on_apply_and_history_not_on_unapplied_drafts()
-> Result<(), WorkbenchError> {
    let document = Document::new(":: before default\n-> END\n")?;
    let mut model = Workbench::from_document(document, Some(View::Source))?;
    model.set_draft(":: after\n-> before\n".into());
    assert_eq!(
        model.document().block_summaries()[0].id().as_str(),
        "before"
    );
    model.apply()?;
    let block = &model.document().block_summaries()[0];
    assert_eq!(block.id().as_str(), "after");
    assert!(!block.is_default());
    assert!(!block.has_end_destination());
    model.undo()?;
    let block = &model.document().block_summaries()[0];
    assert_eq!(block.id().as_str(), "before");
    assert!(block.is_default());
    assert!(block.has_end_destination());
    model.redo()?;
    assert_eq!(model.document().block_summaries()[0].id().as_str(), "after");
    Ok(())
}
