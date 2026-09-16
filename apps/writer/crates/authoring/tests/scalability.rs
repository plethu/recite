use recite_compiler::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, SearchIndex};

#[test]
fn projections_follow_revision_and_unicode_history_retains_only_changed_text()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(recite_writer_model::FIXTURE)?;
    let original = document.source().to_owned();
    let first = document.script_snapshot()?;
    assert!(std::sync::Arc::ptr_eq(&first, &document.script_snapshot()?));
    let changed = original.replace("Would you tell me", "Café — could you tell me");
    document.replace_source(document.revision(), changed.clone())?;
    assert!(!std::sync::Arc::ptr_eq(
        &first,
        &document.script_snapshot()?
    ));
    assert!(document.history_bytes() < 100);
    assert!(document.undo()?);
    assert_eq!(document.source(), original);
    assert!(document.redo()?);
    assert_eq!(document.source(), changed);
    document.replace_source(document.revision(), "broken syntax".into())?;
    assert!(document.undo()?);
    assert_eq!(document.source(), changed);
    Ok(())
}

#[test]
fn search_replaces_one_document_and_combines_speaker_with_unicode_words()
-> Result<(), Box<dyn std::error::Error>> {
    let a = SavedDocument::new(DocumentKey::new("a.recite")?, recite_writer_model::FIXTURE);
    let b = SavedDocument::new(
        DocumentKey::new("b.recite")?,
        recite_writer_model::FIXTURE.replace("Would you tell me", "Café courier"),
    );
    let mut index = SearchIndex::build(&[a, b]);
    let (count, hits) = index.search("alice café", 1);
    assert_eq!(count, 1);
    assert_eq!(hits[0].document, "b.recite");
    index.replace_document(SavedDocument::new(
        DocumentKey::new("b.recite")?,
        recite_writer_model::FIXTURE,
    ));
    assert_eq!(index.search("café", 10).0, 0);
    assert_eq!(index.search("alice would", 1).0, 2);
    assert_eq!(index.search("alice would", 1).1.len(), 1);
    Ok(())
}

#[cfg(feature = "benchmarks")]
#[test]
fn generated_project_has_unique_blocks_ids_and_one_default()
-> Result<(), Box<dyn std::error::Error>> {
    let documents = recite_writer_model::workload::project(1200, 500)?;
    let document = Document::in_project(
        documents[0].key().clone(),
        documents[0].text(),
        recite_writer_model::ProjectContext {
            documents: documents.clone(),
            schema: None,
        },
    )?;
    assert!(
        document.diagnostics().is_empty(),
        "{:?}",
        document.diagnostics()
    );
    Ok(())
}
