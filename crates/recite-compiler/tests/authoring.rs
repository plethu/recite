#![cfg(test)]

use recite_compiler::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentLayer, DocumentVersion,
    OpenDocument, SavedDocument, SnapshotGeneration,
};
use recite_core::DocumentKey;

fn key(value: &str) -> DocumentKey {
    match DocumentKey::new(value) {
        Ok(key) => key,
        Err(error) => panic!("test key is valid: {error}"),
    }
}

fn request(
    generation: SnapshotGeneration,
    saved: impl IntoIterator<Item = SavedDocument>,
    open: impl IntoIterator<Item = OpenDocument>,
) -> AuthoringRequest {
    AuthoringRequest::new(generation, saved, open)
}

fn saved(name: &str, text: &str) -> SavedDocument {
    SavedDocument::new(key(name), text)
}

fn open(name: &str, version: i64, text: &str) -> OpenDocument {
    OpenDocument::new(key(name), DocumentVersion::new(version), text)
}

#[test]
fn saved_and_overlay_inputs_are_sorted_and_overlay_wins() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("z.recite", ":: z\n"), saved("a.recite", ":: a\n")],
            [
                open("z.recite", -2, ":: overlay\n"),
                open("open.recite", i64::MAX, ":: open\n"),
            ],
        ))
        .expect("initial request accepted");

    let documents = kernel.snapshot().documents();
    assert_eq!(documents.len(), 3);
    assert_eq!(documents[0].key().as_str(), "a.recite");
    assert_eq!(documents[1].key().as_str(), "open.recite");
    assert_eq!(documents[2].key().as_str(), "z.recite");
    assert_eq!(documents[2].layer(), DocumentLayer::Open);
    assert_eq!(
        documents[2].version().map(DocumentVersion::as_i64),
        Some(-2)
    );
    assert_eq!(documents[2].metadata().byte_len(), ":: overlay\n".len());
}

#[test]
fn duplicate_layers_and_versions_are_typed_and_transactional() {
    let mut kernel = AuthoringKernel::new();
    let initial = request(
        SnapshotGeneration::initial(),
        [saved("a.recite", ":: a\n")],
        [open("a.recite", 1, ":: open\n")],
    );
    kernel.apply(initial).expect("initial request accepted");
    let before = kernel.snapshot().clone();
    let before_delta = kernel.delta().clone();

    let error = kernel
        .apply(request(
            before.generation(),
            [
                saved("a.recite", ":: a\n"),
                saved("a.recite", ":: duplicate\n"),
            ],
            [],
        ))
        .expect_err("duplicate saved documents fail");
    assert!(matches!(
        error,
        AuthoringError::DuplicateSavedDocument { .. }
    ));
    assert_eq!(kernel.snapshot(), &before);
    assert_eq!(kernel.delta(), &before_delta);

    let error = kernel
        .apply(request(
            before.generation(),
            [saved("a.recite", ":: a\n")],
            [open("a.recite", 1, ":: changed\n")],
        ))
        .expect_err("same-version changed text fails");
    assert!(matches!(
        error,
        AuthoringError::OverlayVersionConflict { .. }
    ));
    assert_eq!(kernel.snapshot(), &before);

    let error = kernel
        .apply(request(
            before.generation(),
            [saved("a.recite", ":: a\n")],
            [open("a.recite", 0, ":: old\n")],
        ))
        .expect_err("decreasing version fails");
    assert!(matches!(error, AuthoringError::StaleOverlayVersion { .. }));
    assert_eq!(kernel.snapshot(), &before);
}

#[test]
fn equal_overlay_input_is_a_no_op_and_close_resets_lifecycle() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [],
            [open("a.recite", i32::MIN as i64, ":: a\n")],
        ))
        .expect("initial request accepted");
    let snapshot = kernel.snapshot().clone();
    let delta = kernel.delta().clone();
    kernel
        .apply(request(
            snapshot.generation(),
            [],
            [open("a.recite", i32::MIN as i64, ":: a\n")],
        ))
        .expect("identical request is accepted as a no-op");
    assert_eq!(kernel.snapshot(), &snapshot);
    assert_eq!(kernel.delta(), &delta);

    kernel
        .apply(request(snapshot.generation(), [], []))
        .expect("close accepted");
    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(
            generation,
            [],
            [open("a.recite", i32::MAX as i64, ":: a\n")],
        ))
        .expect("reopen accepts any first version");
    assert_eq!(
        kernel.snapshot().documents()[0]
            .version()
            .map(DocumentVersion::as_i64),
        Some(i32::MAX as i64)
    );
}

#[test]
fn generation_mismatch_is_transactional() {
    let mut kernel = AuthoringKernel::new();
    let error = kernel
        .apply(request(
            SnapshotGeneration::new(42),
            [saved("a.recite", ":: a\n")],
            [],
        ))
        .expect_err("wrong expected generation fails");
    assert!(matches!(error, AuthoringError::GenerationMismatch { .. }));
    assert_eq!(
        kernel.snapshot().generation(),
        SnapshotGeneration::initial()
    );
    assert!(kernel.snapshot().documents().is_empty());
}

#[test]
fn malformed_document_keeps_parser_diagnostics_without_poisoning_clean_validation() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [
                saved(
                    "clean.recite",
                    concat!(
                        ":: start default\n",
                        ">\n",
                        "  A line without an ID.\n",
                        "-> malformed.recite::missing\n",
                        "! immediate missing_effect()\n",
                    ),
                ),
                saved("malformed.recite", "not a statement\n"),
            ],
            [],
        ))
        .expect("recoverable source request accepted");

    let clean = kernel
        .snapshot()
        .document(&key("clean.recite"))
        .expect("clean document is present");
    let malformed = kernel
        .snapshot()
        .document(&key("malformed.recite"))
        .expect("malformed document is present");
    assert!(
        malformed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str().starts_with("RECITE_PARSE"))
    );
    assert!(!malformed.participation().block_definitions().is_complete());
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_ID001")
    );
    assert!(
        clean
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code.as_str() != "RECITE_VALIDATE007")
    );

    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(
            generation,
            [
                saved(
                    "clean.recite",
                    concat!(
                        ":: start default\n",
                        "> line@11111111111111111111\n",
                        "  A line with an ID.\n",
                        "-> malformed.recite::missing\n",
                    ),
                ),
                saved("malformed.recite", ":: known\n"),
            ],
            [],
        ))
        .expect("complete replacement accepted");
    let clean = kernel
        .snapshot()
        .document(&key("clean.recite"))
        .expect("clean document remains present");
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_VALIDATE007")
    );
}

#[test]
fn delta_contains_sorted_changed_and_removed_metadata() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("z.recite", ":: z\n"), saved("a.recite", ":: a\n")],
            [],
        ))
        .expect("initial request accepted");
    assert_eq!(
        kernel
            .delta()
            .changed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "z.recite"]
    );

    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(generation, [saved("z.recite", ":: changed\n")], []))
        .expect("replacement accepted");
    assert_eq!(
        kernel
            .delta()
            .removed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite"]
    );
    assert_eq!(
        kernel.delta().changed()[0]
            .previous()
            .map(|metadata| metadata.byte_len()),
        Some(":: z\n".len())
    );
    assert_eq!(
        kernel.delta().changed()[0]
            .current()
            .map(|metadata| metadata.byte_len()),
        Some(":: changed\n".len())
    );
}
