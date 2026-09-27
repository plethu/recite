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

#[test]
fn prose_revalidates_one_file_and_reuses_project_diagnostics() {
    super::PROJECT_VALIDATION_COUNT.with(|count| count.set(0));
    ANALYZE_COUNT.with(|count| count.set(0));
    let mut kernel = AuthoringKernel::new();
    let a = ":: a default\n> line@11111111111111111111\n  A\n-> END\n";
    let b = ":: b\n-> missing\n";
    kernel
        .apply(request(kernel.snapshot().generation(), a, b))
        .unwrap();
    let old_b = kernel
        .snapshot()
        .document(&key("b.recite"))
        .unwrap()
        .clone();
    kernel
        .apply(request(
            kernel.snapshot().generation(),
            &a.replace("  A", "  Café with more words"),
            b,
        ))
        .unwrap();
    assert_eq!(ANALYZE_COUNT.with(Cell::get), 3);
    assert_eq!(super::PROJECT_VALIDATION_COUNT.with(Cell::get), 1);
    let next_b = kernel.snapshot().document(&key("b.recite")).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        old_b.shared_diagnostics(),
        next_b.shared_diagnostics()
    ));
    kernel
        .apply(request(
            kernel.snapshot().generation(),
            &a.replace(":: a", ":: missing"),
            b,
        ))
        .unwrap();
    assert_eq!(super::PROJECT_VALIDATION_COUNT.with(Cell::get), 2);
}

#[test]
fn local_markup_errors_change_without_rebuilding_project_indexes() {
    super::PROJECT_VALIDATION_COUNT.with(|count| count.set(0));
    let mut kernel = AuthoringKernel::with_schema(recite_core::schema::ProjectSchema::empty_v1());
    let a = ":: a default\n> line@11111111111111111111\n  Hello.\n-> END\n";
    let b = ":: b\n-> END\n";
    kernel
        .apply(request(kernel.snapshot().generation(), a, b))
        .unwrap();
    assert!(
        kernel
            .snapshot()
            .documents()
            .iter()
            .all(|d| d.diagnostics().is_empty())
    );
    kernel
        .apply(request(
            kernel.snapshot().generation(),
            &a.replace("Hello.", "[unknown]Hello."),
            b,
        ))
        .unwrap();
    assert!(
        !kernel
            .snapshot()
            .document(&key("a.recite"))
            .unwrap()
            .diagnostics()
            .is_empty()
    );
    assert_eq!(super::PROJECT_VALIDATION_COUNT.with(Cell::get), 1);
    kernel
        .apply(request(kernel.snapshot().generation(), a, b))
        .unwrap();
    assert!(
        kernel
            .snapshot()
            .documents()
            .iter()
            .all(|d| d.diagnostics().is_empty())
    );
    assert_eq!(super::PROJECT_VALIDATION_COUNT.with(Cell::get), 1);
}

#[test]
fn shared_destination_does_not_revalidate_unrelated_callers() {
    use crate::validation::incremental::VALIDATED_DOCUMENTS;
    let mut kernel = AuthoringKernel::new();
    let mut saved: Vec<_> = (0..100)
        .map(|n| {
            SavedDocument::new(
                key(&format!("scene{n:03}.recite")),
                format!(
                    ":: beat{n}{}\n> line@{n:020x}\n  Hello.\n-> scene000.recite::beat0\n",
                    if n == 0 { " default" } else { "" }
                ),
            )
        })
        .collect();
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved.clone(),
            [],
        ))
        .unwrap();
    for text in [
        ":: beat50\n> line@ffffffffffffffffffff\n  Hello.\n-> scene000.recite::beat0\n",
        ":: beat50\n> line@ffffffffffffffffffff\n  Hello.\n  More words.\n-> scene000.recite::beat0\n",
    ] {
        VALIDATED_DOCUMENTS.with(|count| count.set(0));
        saved[50] = SavedDocument::new(key("scene050.recite"), text);
        kernel
            .apply(AuthoringRequest::new(
                kernel.snapshot().generation(),
                saved.clone(),
                [],
            ))
            .unwrap();
        assert!(
            VALIDATED_DOCUMENTS.with(Cell::get) <= 3,
            "unrelated scenes must not be validated"
        );
        assert!(
            kernel
                .snapshot()
                .documents()
                .iter()
                .all(|doc| doc.diagnostics().is_empty())
        );
    }
    VALIDATED_DOCUMENTS.with(|count| count.set(0));
    saved[0] = SavedDocument::new(key("scene000.recite"), format!("\n{}", saved[0].text()));
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved.clone(),
            [],
        ))
        .unwrap();
    assert_eq!(
        VALIDATED_DOCUMENTS.with(Cell::get),
        1,
        "moving the destination must not invalidate callers"
    );
    saved[0] = SavedDocument::new(
        key("scene000.recite"),
        saved[0].text().replace(":: beat0", ":: renamed"),
    );
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved,
            [],
        ))
        .unwrap();
    assert!(
        kernel
            .snapshot()
            .documents()
            .iter()
            .all(|doc| !doc.diagnostics().is_empty()),
        "removing the referenced block must invalidate every caller"
    );
}
