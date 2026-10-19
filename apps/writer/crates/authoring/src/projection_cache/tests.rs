use super::*;

#[test]
fn worker_hints_share_only_live_passage_rows_and_their_paired_id_map() {
    let document =
        Document::new(":: start default\n? reply@22222222222222222222\n  Continue.\n  -> END\n")
            .unwrap();
    assert!(document.projections.capture_passages().is_none());
    let passages = document.passage_snapshot().unwrap();
    let no_ids = document.projections.capture_passages().unwrap().restore();
    assert!(Arc::ptr_eq(&passages, no_ids.passages.get().unwrap()));
    assert!(no_ids.ids.get().is_none());
    assert_eq!(
        document
            .reply_rules_request("22222222222222222222")
            .prepare()
            .unwrap()
            .text,
        "Continue."
    );
    drop(no_ids);
    drop(passages);
    document.find_passage("22222222222222222222").unwrap();
    let hint = document.projections.capture_passages().unwrap();
    let cache = hint.restore();
    assert!(Arc::ptr_eq(
        cache.passages.get().unwrap(),
        document.projections.passages.get().unwrap()
    ));
    assert!(Arc::ptr_eq(
        cache.ids.get().unwrap(),
        document.projections.ids.get().unwrap()
    ));
    assert!(cache.script.get().is_none());
    assert!(cache.bindings.get().is_none());
    let ids = document.projections.ids.get().unwrap().clone();
    drop(cache);
    drop(document);
    assert!(hint.passages.upgrade().is_none());
    let cold = hint.restore();
    assert!(cold.passages.get().is_none());
    assert!(cold.ids.get().is_none());
    assert_eq!(ids.len(), 1);
}
