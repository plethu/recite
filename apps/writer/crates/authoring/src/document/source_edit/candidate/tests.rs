use super::*;

#[test]
fn prepared_candidate_reuses_final_analysis_and_source_without_recompiling() {
    let base = Document::new(":: start default\n-> END\n").unwrap();
    let request = base
        .source_edit_request(base.source_snapshot().into(), EditOperation::Apply)
        .unwrap();
    let mut candidate = base.completion_snapshot().projection_document();
    candidate.add_beat(candidate.revision()).unwrap();
    let source = candidate.source_snapshot();
    let kernel = candidate.kernel.clone();
    let version = candidate.revision();
    let prepared = request.prepare_candidate(candidate).unwrap();
    let projected = prepared.projection_document();
    assert!(Arc::ptr_eq(&source, &projected.source_snapshot()));
    assert!(Arc::ptr_eq(&kernel, &projected.kernel));
    assert_eq!(projected.revision(), version);
    assert!(projected.source().contains("new_beat_1"));
}
