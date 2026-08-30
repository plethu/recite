use super::ANALYZE_COUNT;
use crate::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration};
use recite_core::{DocumentKey, SourcePosition};
use std::cell::Cell;

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test key is valid")
}

fn request(generation: SnapshotGeneration, a: &str, b: &str) -> AuthoringRequest {
    AuthoringRequest::new(
        generation,
        [
            SavedDocument::new(key("a.recite"), a),
            SavedDocument::new(key("b.recite"), b),
        ],
        [],
    )
}

#[test]
fn analysis_is_reused_for_queries_and_unchanged_documents() {
    ANALYZE_COUNT.with(|count| count.set(0));
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            ":: a\n> line@11111111111111111111\n  A\n",
            ":: b\n",
        ))
        .expect("initial request accepted");
    assert_eq!(ANALYZE_COUNT.with(Cell::get), 2);

    let a_key = key("a.recite");
    let position = SourcePosition::new(2, 3).expect("valid position");
    let _ = kernel.snapshot().diagnostics();
    let _ = kernel.snapshot().document_diagnostics(&a_key);
    let _ = kernel.snapshot().symbols(&a_key, Default::default());
    let _ = kernel.snapshot().project_symbols(Default::default());
    let _ = kernel.snapshot().complete(&a_key, position);
    let _ = kernel.snapshot().navigate(&a_key, position);
    let _ = kernel
        .snapshot()
        .references(&a_key, position, Default::default());
    let _ = kernel.snapshot().projection_candidates("missing");
    let _ = kernel.snapshot().hover(&a_key, position);
    assert_eq!(ANALYZE_COUNT.with(Cell::get), 2);

    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(
            generation,
            ":: a\n> line@22222222222222222222\n  A\n",
            ":: b\n",
        ))
        .expect("one-document replacement accepted");
    assert_eq!(ANALYZE_COUNT.with(Cell::get), 3);
}
