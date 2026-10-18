use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::SearchIndex;
use std::sync::Arc;

#[test]
fn retained_search_hits_release_replaced_source_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let source: Arc<str> =
        ":: start default\n> line@11111111111111111111\n  Original courier.\n-> END\n".into();
    let old_source = Arc::downgrade(&source);
    let key = DocumentKey::new("scene.recite")?;
    let mut index = SearchIndex::build(&[SavedDocument::from_shared(key.clone(), source)]);
    let retained = index.search_results("courier", 1);
    assert!(old_source.upgrade().is_some());
    index.replace_document(SavedDocument::new(
        key,
        ":: start default\n> line@11111111111111111111\n  Current zeppelin.\n-> END\n",
    ));
    assert!(old_source.upgrade().is_none());
    assert_eq!(
        retained.get(0).ok_or("retained hit")?.text,
        "Original courier."
    );
    assert_eq!(index.search_results("zeppelin", 1).total(), 1);
    drop(index);
    assert_eq!(
        retained.get(0).ok_or("retained hit")?.text,
        "Original courier."
    );
    Ok(())
}
