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

#[test]
fn indexed_replacements_match_scalar_scanner_and_outlive_documents()
-> Result<(), Box<dyn std::error::Error>> {
    for newline in ["\n", "\r\n", "\r"] {
        for token in ["dest", ""] {
            let source = format!(
                ":: destination default\n> line@11111111111111111111\n  Café 🐧 שלום.\n-> {token}"
            )
            .replace('\n', newline);
            let completions = {
                let document = Document::new(&source)?;
                document.complete_draft(
                    &source,
                    SourcePosition::new(4, (4 + token.len()).try_into()?)?,
                )?
            };
            assert!(!completions.candidates().is_empty());
            let independently_owned = source.clone();
            for (index, candidate) in completions.candidates().iter().enumerate() {
                let span = candidate.replace_span();
                let start =
                    recite_core::byte_offset_for_position(&source, span.start).ok_or("start")?;
                let end = if let Some(end) = span.end {
                    let offset =
                        recite_core::byte_offset_for_position(&source, end).ok_or("end")?;
                    offset
                        + source[offset..]
                            .chars()
                            .next()
                            .filter(|c| !matches!(c, '\r' | '\n'))
                            .map_or(0, char::len_utf8)
                } else {
                    start
                };
                assert_eq!(
                    completions.replacement(&independently_owned, index)?,
                    (start..end, candidate.name())
                );
                assert_eq!(
                    completions.replacement(completions.source(), index)?,
                    (start..end, candidate.name())
                );
            }
            let changed_same_length = independently_owned.replacen("Café", "Cafè", 1);
            assert!(matches!(
                completions.replacement(&changed_same_length, usize::MAX),
                Err(recite_writer_model::EditError::Stale)
            ));
            assert!(matches!(
                completions.replacement(&independently_owned, usize::MAX),
                Err(recite_writer_model::EditError::Position)
            ));
        }
    }
    Ok(())
}
