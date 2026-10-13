use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, RecoveredDraft, SearchIndex, View, Workbench};
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

#[test]
fn search_lowercasing_preserves_unicode_and_source_order() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        ":: start default speaker=İSTANBUL\n",
        "> first@00000000000000000001\n",
        "  Straße CAFÉ Beacon17 and courier courier.\n",
        "? reply@00000000000000000002\n",
        "  Couriers wait beside the café.\n",
        "  -> END\n",
    );
    let document = SavedDocument::new(DocumentKey::new("Mixed_Case.recite")?, source);
    let index = SearchIndex::build(&[document]);
    let (count, hits) = index.search("MIXED_CASE CAFÉ", 2);
    assert_eq!(count, 2);
    assert_eq!(hits[0].text, "Straße CAFÉ Beacon17 and courier courier.");
    assert_eq!(hits[1].text, "Couriers wait beside the café.");
    assert_eq!(index.search("straße beacon17 COURIER", 10).0, 1);
    assert_eq!(index.search("İSTANBUL café", 10).0, 1);
    assert_eq!(index.search("istanbul", 10).0, 0);
    assert_eq!(index.search("STRASSE", 10).0, 0);
    Ok(())
}

#[test]
fn shared_search_results_preserve_order_counts_and_their_saved_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> first@00000000000000000001 speaker=mara\n  Café courier.\n> second@00000000000000000002\n  Another café.\n-> END\n";
    let documents = ["a.recite", "b.recite"]
        .into_iter()
        .map(|key| Ok(SavedDocument::new(DocumentKey::new(key)?, source)))
        .collect::<Result<Vec<_>, recite_core::DocumentKeyError>>()?;
    let mut index = SearchIndex::build(&documents);
    for query in ["CAFÉ café", "café mara", "café missing", "..."] {
        for limit in [0, 1, 3, 100] {
            let owned = index.search(query, limit);
            let shared = index.search_results(query, limit);
            assert_eq!(shared.total(), owned.0);
            assert_eq!(shared.len(), owned.1.len());
            for (position, hit) in owned.1.iter().enumerate() {
                assert_eq!(shared.get(position), Some(hit));
            }
            assert!(shared.get(shared.len()).is_none());
        }
    }
    let snapshot = index.search_results("café", 3);
    let cloned = snapshot.clone();
    assert!(snapshot == cloned);
    assert!(std::ptr::eq(
        snapshot.get(0).ok_or("first search result")?,
        cloned.get(0).ok_or("cloned search result")?
    ));
    index.replace_document(SavedDocument::new(
        documents[0].key().clone(),
        source.replace("café", "tea").replace("Café", "Tea"),
    ));
    assert_eq!(index.search_results("café", 100).total(), 2);
    drop(index);
    assert_eq!(snapshot.total(), 4);
    assert_eq!(
        snapshot.get(0).ok_or("first search result")?.text,
        "Café courier."
    );
    assert_eq!(
        snapshot.get(2).ok_or("third search result")?.document,
        "b.recite"
    );
    Ok(())
}

#[test]
fn recovery_shares_immutable_drafts_and_preserves_the_existing_json_shape()
-> Result<(), Box<dyn std::error::Error>> {
    let mut workbench = Workbench::new(recite_writer_model::FIXTURE)?;
    workbench.select(View::Source)?;
    let draft = "An incomplete café draft.\n".repeat(10_000);
    let pointer = draft.as_ptr();
    workbench.set_draft(draft.clone());
    let expected = serde_json::json!({
        "source": recite_writer_model::FIXTURE,
        "view": "Source",
        "draft": draft,
    });
    let legacy: RecoveredDraft = serde_json::from_value(expected.clone())?;
    assert_eq!(serde_json::to_value(workbench.recovery())?, expected);
    assert_eq!(legacy, workbench.recovery());
    workbench.set_draft(draft);
    assert_eq!(workbench.draft().as_ptr(), pointer);
    let captured = workbench.recovery();
    workbench.set_draft("A later draft".into());
    workbench.discard();
    captured.restore(&mut workbench)?;
    assert_eq!(workbench.draft().as_ptr(), pointer);
    assert!(workbench.has_draft());
    assert_eq!(serde_json::to_value(&captured)?, expected);
    workbench.discard();
    workbench.set_draft(workbench.draft().to_owned());
    assert!(
        !workbench.has_draft(),
        "equal independently allocated drafts stay clean"
    );
    Ok(())
}
