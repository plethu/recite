use super::SearchSnapshot;
use recite_writer_model::SearchIndex;
use std::sync::Arc;

#[test]
fn search_memo_tracks_identity_without_retaining_displaced_indexes() {
    let index = Arc::new(SearchIndex::default());
    let snapshot = SearchSnapshot(Some(Arc::downgrade(&index)));
    assert!(snapshot == snapshot.clone());
    let replacement = Arc::new(SearchIndex::default());
    assert!(snapshot != SearchSnapshot(Some(Arc::downgrade(&replacement))));
    assert_eq!(Arc::strong_count(&index), 1);
    drop(index);
    assert!(snapshot.0.as_ref().unwrap().upgrade().is_none());
}
