#![cfg(test)]

use recite_compiler::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentLayer, DocumentVersion,
    OpenDocument, SavedDocument, SnapshotGeneration,
};
use recite_core::DocumentKey;

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test key is valid")
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
    assert_eq!(
        documents
            .iter()
            .map(|document| document.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "open.recite", "z.recite"]
    );
    assert_eq!(documents[2].layer(), DocumentLayer::Open);
    assert_eq!(
        documents[2].version().map(DocumentVersion::as_i64),
        Some(-2)
    );
    assert_eq!(documents[2].metadata().byte_len(), ":: overlay\n".len());

    kernel
        .apply(request(
            kernel.snapshot().generation(),
            [saved("z.recite", ":: saved\n"), saved("a.recite", ":: a\n")],
            [],
        ))
        .expect("closing overlays falls back to saved inputs");
    assert_eq!(
        kernel
            .snapshot()
            .document(&key("z.recite"))
            .map(|document| document.layer()),
        Some(DocumentLayer::Saved)
    );
    assert!(kernel.snapshot().document(&key("open.recite")).is_none());
}

#[test]
fn duplicate_layers_and_versions_are_typed_and_transactional() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("a.recite", ":: a\n")],
            [open("a.recite", 1, ":: open\n")],
        ))
        .expect("initial request accepted");
    let before = kernel.snapshot().clone();
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
    let no_op_delta = kernel
        .apply(request(
            snapshot.generation(),
            [],
            [open("a.recite", i32::MIN as i64, ":: a\n")],
        ))
        .expect("identical request is accepted as a no-op");
    assert_eq!(kernel.snapshot(), &snapshot);
    assert!(no_op_delta.is_empty());
    assert_eq!(no_op_delta.previous_generation(), snapshot.generation());
    assert_eq!(no_op_delta.generation(), snapshot.generation());

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
