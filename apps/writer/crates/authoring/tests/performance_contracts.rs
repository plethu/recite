use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, SearchIndex};
use std::sync::Arc;

#[test]
fn repeated_words_across_fields_and_query_do_not_duplicate_search_hits()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: courier default\n> first@00000000000000000001 speaker=courier\n  Courier café courier.\n> second@00000000000000000002\n  Café courier.\n-> END\n";
    let documents = ["courier.recite", "other.recite"]
        .into_iter()
        .map(|key| Ok(SavedDocument::new(DocumentKey::new(key)?, source)))
        .collect::<Result<Vec<_>, recite_core::DocumentKeyError>>()?;
    let index = SearchIndex::build(&documents);
    let (count, hits) = index.search("COURIER café courier CAFÉ", 3);
    assert_eq!(count, 4);
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].document, "courier.recite");
    assert_eq!(hits[0].text, "Courier café courier.");
    assert_eq!(hits[1].text, "Café courier.");
    assert_eq!(hits[2].document, "other.recite");
    assert_eq!(index.search("courier", 0), (4, vec![]));
    assert_eq!(index.search("courier missing", 10), (0, vec![]));
    assert_eq!(index.search("...", 10), (0, vec![]));
    Ok(())
}

#[test]
fn cold_script_and_passage_projections_agree_across_edit_and_undo()
-> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::FIXTURE;
    let mut script_first = Document::new(source)?;
    let passage_first = Document::new(source)?;
    let expected = passage_first.passage_snapshot()?;
    script_first.script_snapshot()?;
    let original = script_first.passage_snapshot()?;
    assert_eq!(original, expected);
    assert!(Arc::ptr_eq(&original, &script_first.passage_snapshot()?));
    let edited = source.replace("Would you tell me", "Café — could you tell me");
    script_first.replace_source(script_first.revision(), edited.clone())?;
    script_first.script_snapshot()?;
    let changed = script_first.passage_snapshot()?;
    assert_eq!(changed, Document::new(&edited)?.passage_snapshot()?);
    assert!(!Arc::ptr_eq(&original, &changed));
    assert!(script_first.undo()?);
    script_first.script_snapshot()?;
    assert_eq!(script_first.passage_snapshot()?, original);
    Ok(())
}
