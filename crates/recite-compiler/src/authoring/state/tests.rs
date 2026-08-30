use super::{AuthoringError, AuthoringKernel, SnapshotGeneration};
use crate::{AuthoringRequest, SavedDocument};
use recite_core::DocumentKey;

#[test]
fn exhausted_generation_rejects_without_changing_owned_state() {
    let mut kernel = AuthoringKernel::new();
    let generation = SnapshotGeneration::new(u64::MAX);
    kernel.snapshot = super::super::snapshot::AuthoringSnapshot::new(generation, Vec::new(), None);
    let snapshot = kernel.snapshot.clone();

    let key = match DocumentKey::new("a.recite") {
        Ok(key) => key,
        Err(error) => panic!("test key is valid: {error}"),
    };
    let request = AuthoringRequest::new(
        generation,
        [SavedDocument::new(key, ":: a\n")],
        std::iter::empty(),
    );
    assert!(matches!(
        kernel.apply(request),
        Err(AuthoringError::GenerationExhausted { .. })
    ));
    assert_eq!(kernel.snapshot, snapshot);
    assert!(kernel.saved.is_empty());
    assert!(kernel.analyses.is_empty());
}
