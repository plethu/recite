use recite_core::SourcePosition;
use recite_writer_model::Document;

#[test]
fn completion_queries_the_draft_and_refuses_a_stale_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(":: start default\n-> END\n")?;
    let draft = ":: start default\n-> dest\n:: destination\n-> END\n";
    let completions = document.complete_draft(draft, SourcePosition::new(2, 8)?)?;
    let index = completions
        .candidates()
        .iter()
        .position(|c| c.name() == "destination")
        .ok_or("destination")?;
    let (range, replacement) = completions.replacement(draft, index)?;
    let mut completed = draft.to_owned();
    completed.replace_range(range, replacement);
    assert!(completed.contains("-> destination\n"), "{completed:?}");
    assert!(completions.replacement(&completed, index).is_err());
    assert_eq!(document.source(), ":: start default\n-> END\n");
    Ok(())
}

#[test]
fn draft_queries_match_fresh_project_analysis_without_mutating_accepted_state()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_compiler::authoring::SavedDocument;
    use recite_core::DocumentKey;
    use recite_writer_model::ProjectContext;
    let source = ":: start default\n> line@11111111111111111111\n  Café 🐧.\n-> destination\n:: destination\n-> END\n";
    let key = DocumentKey::new("active.recite")?;
    let context = ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("other.recite")?,
            ":: other\n-> END\n",
        )],
        schema: None,
    };
    let document = Document::in_project(key.clone(), source, context.clone())?;
    let before = document.source_snapshot();
    let revision = document.revision();
    let diagnostics = document.diagnostics();
    for draft in [
        source.to_owned(),
        source.replace("destination", "dest"),
        source.replace("Café 🐧.", "Longer مرحبا.\n  More prose."),
        source.replace("-> destination", "-> missing"),
    ] {
        let fresh = Document::in_project(key.clone(), &draft, context.clone())?;
        for line in 1..=draft.lines().count() {
            let position = SourcePosition::new(u32::try_from(line)?, 1)?;
            assert_eq!(
                document.complete_draft(&draft, position)?,
                fresh.complete_draft(&draft, position)?
            );
        }
    }
    assert!(std::sync::Arc::ptr_eq(&before, &document.source_snapshot()));
    assert_eq!(document.revision(), revision);
    assert_eq!(document.diagnostics(), diagnostics);
    assert_eq!(document.history_bytes(), 0);
    Ok(())
}
