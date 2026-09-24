use recite_writer_model::{Document, EditError, FIXTURE, PassageKind, Preview};

const ALICE: &str = "7701ceab59d2adfa057a";
const CHOICE: &str = "a6f46c2edbe8466b9bfd";

#[test]
fn speaker_edit_changes_the_effective_duplicate_assignment()
-> Result<(), Box<dyn std::error::Error>> {
    let source = FIXTURE.replacen("speaker=alice", "speaker=alice speaker=cheshire_cat", 1);
    let mut doc = Document::new(&source)?;
    doc.set_speaker(doc.revision(), ALICE, "alice")?;
    assert!(
        matches!(&doc.passages()?[0].kind, PassageKind::Dialogue { speaker: Some(speaker) } if speaker == "alice")
    );
    assert_eq!(
        doc.source(),
        source.replacen(
            "speaker=alice speaker=cheshire_cat",
            "speaker=alice speaker=alice",
            1
        )
    );
    Ok(())
}

#[test]
fn drafts_guard_context_and_undo_restores_invalid_source_safely()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_writer_model::{View, Workbench, WorkbenchError};
    let mut model = Workbench::new(FIXTURE)?;
    model.start_preview()?;
    model.set_draft("A new question.".into());
    assert!(model.preview_stale());
    assert!(matches!(
        model.select(View::Source),
        Err(WorkbenchError::DraftPending)
    ));
    assert!(matches!(model.undo(), Err(WorkbenchError::DraftPending)));
    assert_eq!(model.draft(), "A new question.");
    model.discard();
    assert!(!model.preview_stale());
    model.select(View::Source)?;
    let invalid = ":: bad\n> bad@bad\n  Keep me.\n";
    model.set_draft(invalid.into());
    model.apply()?;
    assert!(model.preview_stale());
    model.set_draft(FIXTURE.into());
    model.apply()?;
    model.select(View::Passage(ALICE.into()))?;
    model.undo()?;
    assert_eq!(model.view(), &View::Source);
    assert_eq!(model.draft(), invalid);
    model.redo()?;
    assert_eq!(model.draft(), FIXTURE);
    Ok(())
}

#[test]
fn multiline_unicode_edits_preserve_ids_comments_and_crlf() -> Result<(), Box<dyn std::error::Error>>
{
    let original = FIXTURE.replace('\n', "\r\n");
    let mut doc = Document::new(&original)?;
    let before = doc.passages()?;
    doc.replace_text(doc.revision(), ALICE, "Café 💬\nمرحبا")?;
    let after = doc.passages()?;
    assert_eq!(
        after.iter().map(|p| &p.id).collect::<Vec<_>>(),
        before.iter().map(|p| &p.id).collect::<Vec<_>>()
    );
    assert!(doc.source().contains("  Café 💬\r\n  مرحبا\r\n"));
    assert!(
        doc.source()
            .contains("# Keep this comment and every existing ID")
    );
    assert!(doc.source().contains("portrait=grin"));
    assert_eq!(doc.diagnostics().len(), 0);
    assert!(doc.undo()?);
    assert_eq!(doc.source(), original);
    assert!(doc.redo()?);
    assert_eq!(
        doc.passages()?.first().map(|p| p.text.as_str()),
        Some("Café 💬\nمرحبا")
    );
    Ok(())
}

#[test]
fn prose_cannot_inject_control_flow_and_stale_edits_are_refused()
-> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new(FIXTURE)?;
    for text in [
        "Hello\n-> END",
        "> other@33333333333333333333\nHello",
        "Hello\n",
        "",
    ] {
        assert!(matches!(
            doc.replace_text(doc.revision(), ALICE, text),
            Err(EditError::NotProse)
        ));
        assert_eq!(doc.source(), FIXTURE);
    }
    let revision = doc.revision();
    doc.replace_text(revision, ALICE, "Hello.")?;
    assert!(matches!(
        doc.replace_text(revision, ALICE, "Lost update"),
        Err(EditError::Stale)
    ));
    assert_eq!(
        doc.passages()?.first().map(|p| p.text.as_str()),
        Some("Hello.")
    );
    Ok(())
}

#[test]
fn speaker_and_destination_edits_touch_only_the_selected_field()
-> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new(FIXTURE)?;
    doc.set_speaker(doc.revision(), ALICE, "cheshire_cat")?;
    assert_eq!(
        doc.source(),
        FIXTURE.replacen("speaker=alice", "speaker=cheshire_cat", 1)
    );
    doc.set_destination(doc.revision(), CHOICE, "garden")?;
    let passage = doc.passages()?.into_iter().find(|p| p.id == CHOICE);
    assert!(
        matches!(passage.map(|p| p.kind), Some(PassageKind::Choice { destination: Some(target) }) if target == "garden")
    );
    assert!(matches!(
        doc.set_destination(doc.revision(), CHOICE, "missing"),
        Err(EditError::Destination)
    ));
    Ok(())
}

#[test]
fn added_choice_uses_the_kernel_id_planner_and_one_undo_transaction()
-> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new(FIXTURE)?;
    let old = doc.passages()?;
    let added = doc.add_choice(doc.revision(), "which_way")?;
    let passages = doc.passages()?;
    assert_eq!(passages.len(), old.len() + 1);
    assert!(old.iter().all(|p| {
        passages
            .iter()
            .any(|new| new.id == p.id && new.text == p.text)
    }));
    assert!(
        passages
            .iter()
            .any(|p| p.id == added && p.text == "New choice.")
    );
    assert!(doc.diagnostics().is_empty(), "{:?}", doc.diagnostics());
    assert!(doc.undo()?);
    assert_eq!(doc.source(), FIXTURE);
    Ok(())
}

#[test]
fn invalid_source_survives_and_preview_uses_the_real_choice_path()
-> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new(FIXTURE)?;
    let mut preview = Preview::new(&doc)?;
    assert!(preview.advance(None)?.text.contains("Would you tell me"));
    assert!(preview.advance(None)?.text.contains("That depends"));
    let prompt = preview.advance(None)?;
    assert_eq!(prompt.choices.len(), 2);
    let choice = prompt
        .choices
        .iter()
        .find(|choice| choice.text.contains("garden"))
        .ok_or("garden choice missing")?;
    assert!(
        preview
            .advance(Some(choice.id.clone()))?
            .text
            .contains("Then I shall")
    );
    assert!(preview.advance(None)?.ended);
    doc.replace_source(
        doc.revision(),
        ":: bad\n> wrong@bad\n  Keep my draft.\n".to_owned(),
    )?;
    assert!(doc.source().contains("Keep my draft."));
    assert!(!doc.diagnostics().is_empty());
    assert!(doc.passages().is_err());
    assert!(Preview::new(&doc).is_err());
    assert_ne!(preview.revision(), doc.revision());
    Ok(())
}

#[test]
fn real_document_names_separate_generated_choice_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut first = recite_writer_model::Workbench::open("first.recite", FIXTURE)?;
    let mut second = recite_writer_model::Workbench::open("second.recite", FIXTURE)?;
    first.add_choice()?;
    second.add_choice()?;
    let first_id = first.selected()?.ok_or("first choice")?.id;
    let second_id = second.selected()?.ok_or("second choice")?.id;
    assert_ne!(first_id, second_id);
    let invalid = recite_writer_model::Workbench::open("invalid.recite", ":: bad\n> broken@bad\n")?;
    assert_eq!(invalid.view(), &recite_writer_model::View::Source);
    Ok(())
}
