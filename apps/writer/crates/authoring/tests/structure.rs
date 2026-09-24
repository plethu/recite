use recite_writer_model::{PassageKind, View, WRITER_EXAMPLES};

#[test]
fn additions_are_unique_and_undoable_without_rewriting_existing_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let mut session = WRITER_EXAMPLES[2].open()?;
    let original = session.document().source().to_owned();
    let ids: Vec<_> = session
        .document()
        .passages()?
        .into_iter()
        .map(|p| p.id)
        .collect();
    session.add_beat()?;
    assert_eq!(session.selected_block()?.as_deref(), Some("new_beat_1"));
    let added = session.document().source().to_owned();
    session.undo()?;
    assert_eq!(session.document().source(), original);
    session.redo()?;
    assert_eq!(session.document().source(), added);
    session.add_beat()?;
    assert_eq!(session.selected_block()?.as_deref(), Some("new_beat_2"));
    for id in ids {
        assert!(session.document().passages()?.iter().any(|p| p.id == id));
    }
    Ok(())
}

#[test]
fn adding_a_reply_preserves_the_continuation_and_can_be_rewired()
-> Result<(), Box<dyn std::error::Error>> {
    let mut session = WRITER_EXAMPLES[2].open()?;
    let original = session.document().source().to_owned();
    session.add_choice()?;
    let reply = session.selected()?.ok_or("reply")?;
    assert!(
        matches!(reply.kind, PassageKind::Choice { destination: Some(ref target) } if target == "END")
    );
    assert!(!session.document().source().contains(
        "
-> END"
    ));
    session.attribute("last_tram")?;
    assert!(session.document().source().contains("  -> last_tram"));
    session.undo()?;
    session.undo()?;
    assert_eq!(session.document().source(), original);
    Ok(())
}

#[test]
fn line_insertions_precede_branching_and_survive_source_roundtrip()
-> Result<(), Box<dyn std::error::Error>> {
    let mut session = WRITER_EXAMPLES[0].open()?;
    session.add_line()?;
    session.set_draft("One more thing.".into());
    session.apply()?;
    let source = session.document().source();
    assert!(
        source.find("One more thing.").ok_or("new prose")?
            < source.find("? ask_work").ok_or("choice")?
    );
    let inserted = source.to_owned();
    session.select(View::Source)?;
    session.show_script()?;
    assert_eq!(session.document().source(), inserted);
    Ok(())
}

#[test]
fn renaming_updates_incoming_returns_but_keeps_passage_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let mut session = WRITER_EXAMPLES[0].open()?;
    let original = session.document().source().to_owned();
    let ids: Vec<_> = session
        .document()
        .passages()?
        .into_iter()
        .map(|p| p.id)
        .collect();
    session.rename_beat("front_desk")?;
    assert!(session.document().source().contains(":: front_desk"));
    assert!(session.document().source().contains("-> front_desk"));
    assert!(!session.document().source().contains("-> relay_desk"));
    assert_eq!(
        session
            .document()
            .passages()?
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        ids
    );
    session.undo()?;
    assert_eq!(session.document().source(), original);
    assert_eq!(session.selected_block()?.as_deref(), Some("relay_desk"));
    Ok(())
}

#[test]
fn linear_continuation_can_connect_a_new_beat_without_a_choice()
-> Result<(), Box<dyn std::error::Error>> {
    let mut session = WRITER_EXAMPLES[2].open()?;
    session.add_beat()?;
    session.inspect_block("last_tram")?;
    session.set_continuation("new_beat_1")?;
    assert!(session.document().source().contains("\n-> new_beat_1"));
    assert!(
        !session
            .document()
            .passages()?
            .iter()
            .any(|p| matches!(p.kind, PassageKind::Choice { .. }))
    );
    session.undo()?;
    assert!(!session.document().source().contains("-> new_beat_1"));
    Ok(())
}

#[test]
fn generated_beat_names_respect_other_project_documents() -> Result<(), Box<dyn std::error::Error>>
{
    use recite_compiler::SavedDocument;
    use recite_core::DocumentKey;
    use recite_writer_model::{Document, ProjectContext, Workbench};

    let other = ":: new_beat_1\n-> END\n\n:: new_beat_3\n-> END\n";
    let context = ProjectContext {
        documents: vec![SavedDocument::new(DocumentKey::new("other.recite")?, other)],
        schema: None,
    };
    let document = Document::in_project(
        DocumentKey::new("current.recite")?,
        ":: start default\n-> END\n",
        context,
    )?;
    let mut session = Workbench::from_document(document)?;
    for expected in ["new_beat_2", "new_beat_4"] {
        session.add_beat()?;
        assert_eq!(session.selected_block()?.as_deref(), Some(expected));
        assert!(
            session.document().diagnostics().is_empty(),
            "{:?}",
            session.document().diagnostics()
        );
    }
    session.undo()?;
    session.add_beat()?;
    assert_eq!(session.selected_block()?.as_deref(), Some("new_beat_4"));
    session.start_preview()?;
    Ok(())
}

#[test]
fn inserted_anchor_avoids_ids_in_other_documents() -> Result<(), Box<dyn std::error::Error>> {
    use recite_compiler::SavedDocument;
    use recite_core::DocumentKey;
    use recite_writer_model::{Document, ProjectContext};
    let mut document = Document::open(
        DocumentKey::new("current.recite")?,
        ":: start default\n-> END\n",
    )?;
    let first = document.add_line(document.revision(), "start")?;
    document.undo()?;
    document.refresh_project(ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("other.recite")?,
            format!(":: elsewhere\n> other@{first}\n  Existing.\n-> END\n"),
        )],
        schema: None,
    })?;
    let second = document.add_line(document.revision(), "start")?;
    assert_ne!(first, second);
    assert!(
        document.diagnostics().is_empty(),
        "{:?}",
        document.diagnostics()
    );
    document.undo()?;
    assert_eq!(document.add_line(document.revision(), "start")?, second);
    Ok(())
}
