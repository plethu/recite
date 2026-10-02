use super::{AuthoringError, AuthoringKernel, SnapshotGeneration};
use crate::authoring::CancellationToken;
use crate::{authoring::AuthoringRequest, authoring::SavedDocument};
use recite_core::DocumentKey;

#[test]
fn exhausted_generation_rejects_without_changing_owned_state() {
    let mut kernel = AuthoringKernel::new();
    let generation = SnapshotGeneration::new(u64::MAX);
    kernel.snapshot =
        super::super::snapshot::AuthoringSnapshot::new(generation, Vec::new(), None, true);
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

#[test]
fn interruption_at_every_analysis_checkpoint_preserves_committed_state() {
    use crate::authoring::{DocumentVersion, OpenDocument};
    let a = DocumentKey::new("a.recite").expect("key");
    let b = DocumentKey::new("b.recite").expect("key");
    let saved = vec![
        SavedDocument::new(a.clone(), ":: a\n-> b.recite::b\n"),
        SavedDocument::new(b.clone(), ":: b\n-> END\n"),
    ];
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved.clone(),
            [],
        ))
        .expect("initial analysis");
    let snapshot = kernel.snapshot().clone();
    let request = AuthoringRequest::new(
        snapshot.generation(),
        saved,
        [OpenDocument::new(
            b,
            DocumentVersion::new(1),
            ":: renamed\n-> END\n",
        )],
    );
    let expected = kernel
        .updated(request.clone(), &CancellationToken::new())
        .expect("candidate");
    let mut interrupted = 0;
    for cutoff in 0..256 {
        let control = CancellationToken::interrupt_after(cutoff);
        match kernel.apply_with_control(request.clone(), &control) {
            Err(AuthoringError::Interrupted(_)) => {
                interrupted += 1;
                assert_eq!(kernel.snapshot(), &snapshot, "checkpoint {cutoff}");
                assert!(kernel.open.is_empty());
                let retry = kernel
                    .updated(request.clone(), &CancellationToken::new())
                    .expect("retry after interruption");
                assert_eq!(retry.snapshot(), expected.snapshot());
                assert_eq!(retry.analyses, expected.analyses);
                assert_eq!(retry.project_diagnostics, expected.project_diagnostics);
            }
            Ok(_) => {
                assert_eq!(kernel.snapshot(), expected.snapshot());
                break;
            }
            Err(error) => panic!("unexpected error: {error}"),
        }
    }
    assert!(
        interrupted > 10,
        "exercise analysis, indexing, document assembly, and commit checkpoints"
    );
    assert_eq!(
        kernel.snapshot(),
        expected.snapshot(),
        "eventually reaches commit"
    );
}
