#![cfg(test)]

use recite_compiler::{AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration};
use recite_core::DocumentKey;

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test key is valid")
}

#[test]
fn delta_contains_sorted_changed_and_removed_metadata() {
    let mut kernel = AuthoringKernel::new();
    let initial_delta = kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("z.recite"), ":: z\n"),
                SavedDocument::new(key("a.recite"), ":: a\n"),
            ],
            [],
        ))
        .expect("initial request accepted");
    assert_eq!(
        initial_delta
            .changed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "z.recite"]
    );
    let delta = kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(key("z.recite"), ":: changed\n")],
            [],
        ))
        .expect("replacement accepted");
    assert_eq!(
        delta
            .removed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite"]
    );
    assert_eq!(
        delta.changed()[0]
            .previous()
            .map(|metadata| metadata.byte_len()),
        Some(":: z\n".len())
    );
    assert_eq!(
        delta.changed()[0]
            .current()
            .map(|metadata| metadata.byte_len()),
        Some(":: changed\n".len())
    );
}
