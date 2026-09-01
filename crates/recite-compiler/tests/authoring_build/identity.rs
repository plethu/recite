use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildCheck, BuildGeneration, BuildInput, BuildInputPolicy, BuildRequest,
};

#[test]
fn authority_source_is_identity_even_when_overlay_bytes_match_saved_bytes() {
    let saved_request = BuildRequest::new_with_policy(
        BuildGeneration::new(5),
        recite_compiler::SnapshotGeneration::new(5),
        [BuildInput::saved_source(key("a.recite"), "same")],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("saved request: {error}"));
    let overlay_request = BuildRequest::new_with_policy(
        BuildGeneration::new(5),
        recite_compiler::SnapshotGeneration::new(5),
        [BuildInput::overlay_source(key("a.recite"), "same")],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("overlay request: {error}"));
    assert_eq!(
        saved_request.inputs()[0].fingerprint(),
        overlay_request.inputs()[0].fingerprint()
    );
    assert_ne!(saved_request.fingerprints(), overlay_request.fingerprints());
    assert_ne!(
        recite_compiler::BuildRequestIdentity::from_request(&saved_request),
        recite_compiler::BuildRequestIdentity::from_request(&overlay_request)
    );

    struct SavedCheck {
        check: BuildCheck,
    }
    impl recite_compiler::BuildEngine for SavedCheck {
        fn check(&mut self, _: &BuildRequest, _: &recite_compiler::BuildControl) -> BuildCheck {
            self.check.clone()
        }
        fn build(
            &mut self,
            _: &BuildRequest,
            _: &recite_compiler::BuildControl,
        ) -> Result<Vec<recite_compiler::BuildCandidate>, recite_compiler::BuildFailure> {
            Ok(vec![candidate("a.recitec", b"overlay")])
        }
    }
    let mut check_engine = SavedCheck {
        check: BuildCheck::passed(&saved_request),
    };
    let mut check_publisher = FakePublisher::new();
    let check_result = run(
        overlay_request.clone(),
        &recite_compiler::BuildControl::new(),
        &mut check_engine,
        &mut check_publisher,
    );
    assert!(matches!(
        check_result.failure(),
        Some(recite_compiler::BuildResultFailure::Check(
            recite_compiler::BuildCheckError::RequestMismatch
        ))
    ));
    assert_eq!(check_publisher.commit_calls, 0);

    let fence =
        recite_compiler::BuildAuthorityFence::new(BuildAuthority::from_request(&saved_request));
    let mut engine = FakeEngine::new([candidate("a.recitec", b"overlay")]);
    let mut publisher = FakePublisher::new();
    let result = recite_compiler::BuildCoordinator::with_fence(fence)
        .run(
            overlay_request,
            &recite_compiler::BuildControl::new(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("authority refusal: {error}"));
    assert!(matches!(
        result.publish(),
        recite_compiler::PublishOutcome::Refused {
            reason: recite_compiler::PublishRefusal::StaleFingerprints
        }
    ));
    assert_eq!(publisher.commit_calls, 0);
}
