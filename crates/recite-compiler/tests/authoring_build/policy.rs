use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildCandidate, BuildCheck, BuildGeneration, BuildInput, BuildInputAuthority,
    BuildInputKind, BuildInputPayload, BuildInputPolicy, BuildLifecycle, BuildRequest, BuildState,
    BuildTelemetry, BuildTransition, BuildTransitionError, PreparedPublishIdentity,
    RestartGuidance,
};
use std::time::Duration;

#[test]
fn request_requires_explicit_overlays_and_has_stable_order() {
    let overlay = BuildInput::overlay_source(key("z.recite"), "overlay");
    assert!(matches!(
        BuildRequest::new(
            BuildGeneration::initial(),
            recite_compiler::SnapshotGeneration::initial(),
            [overlay]
        ),
        Err(recite_compiler::BuildRequestError::OverlayNotAllowed { .. })
    ));
    let request = BuildRequest::new_with_policy(
        BuildGeneration::new(1),
        recite_compiler::SnapshotGeneration::new(1),
        [
            BuildInput::saved_source(key("z.recite"), "saved"),
            BuildInput::overlay_source(key("z.recite"), "overlay"),
            BuildInput::saved_source(key("a.recite"), "a"),
        ],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("explicit overlay request: {error}"));
    assert_eq!(
        request
            .inputs()
            .iter()
            .map(|input| input.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "z.recite"]
    );
    assert_eq!(request.inputs()[0].content(), Some("a"));
}

#[test]
fn input_order_does_not_change_candidates_or_fingerprints() {
    let left = make_request(
        2,
        [
            BuildInput::saved_source(key("z.recite"), "z"),
            BuildInput::saved_source(key("a.recite"), "a"),
        ],
    );
    let right = make_request(
        2,
        [
            BuildInput::saved_source(key("a.recite"), "a"),
            BuildInput::saved_source(key("z.recite"), "z"),
        ],
    );
    let mut left_engine =
        FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut left_publisher = FakePublisher::new();
    let mut right_engine =
        FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut right_publisher = FakePublisher::new();
    let left_result = run(
        left,
        &recite_compiler::BuildControl::new(),
        &mut left_engine,
        &mut left_publisher,
    );
    let right_result = run(
        right,
        &recite_compiler::BuildControl::new(),
        &mut right_engine,
        &mut right_publisher,
    );
    assert_eq!(left_result.fingerprints(), right_result.fingerprints());
    assert_eq!(left_result.candidates(), right_result.candidates());
}

#[test]
fn schema_payload_has_one_authority_and_canonical_fingerprint() {
    let raw = BuildInput::new(
        key("schema.toml"),
        BuildInputKind::Schema,
        BuildInputAuthority::Saved,
        BuildInputPayload::Text("schema_version = 1".to_owned()),
    );
    assert!(matches!(
        BuildRequest::new(
            BuildGeneration::new(1),
            recite_compiler::SnapshotGeneration::new(1),
            [raw]
        ),
        Err(recite_compiler::BuildRequestError::SchemaPayloadMismatch { .. })
    ));
    let model = recite_core::ProjectSchema::empty_v1();
    let left = BuildRequest::new(
        BuildGeneration::new(2),
        recite_compiler::SnapshotGeneration::new(2),
        [BuildInput::schema(
            key("schema"),
            BuildInputAuthority::Saved,
            model.clone(),
        )],
    )
    .unwrap_or_else(|error| panic!("schema request: {error}"));
    let right = BuildRequest::new(
        BuildGeneration::new(2),
        recite_compiler::SnapshotGeneration::new(2),
        [BuildInput::schema(
            key("schema"),
            BuildInputAuthority::Saved,
            model,
        )],
    )
    .unwrap_or_else(|error| panic!("schema request: {error}"));
    assert_eq!(
        left.fingerprints().schema(),
        &recite_core::ProjectSchema::canonical_fingerprint(&recite_core::ProjectSchema::empty_v1())
    );
    assert_eq!(left.fingerprints().schema(), right.fingerprints().schema());
    assert_eq!(left, right);
    let second = BuildInput::schema(
        key("other-schema"),
        BuildInputAuthority::Saved,
        recite_core::ProjectSchema::empty_v1(),
    );
    assert!(matches!(
        BuildRequest::new(
            BuildGeneration::new(3),
            recite_compiler::SnapshotGeneration::new(3),
            [
                BuildInput::schema(
                    key("schema"),
                    BuildInputAuthority::Saved,
                    recite_core::ProjectSchema::empty_v1()
                ),
                second
            ]
        ),
        Err(recite_compiler::BuildRequestError::MultipleSchemaInputs)
    ));
}

#[test]
fn authority_includes_policy_when_payload_bytes_match() {
    let saved = BuildInput::saved_source(key("a.recite"), "a");
    let strict = make_request(4, [saved.clone()]);
    let permissive = BuildRequest::new_with_policy(
        BuildGeneration::new(4),
        recite_compiler::SnapshotGeneration::new(4),
        [saved],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("policy request: {error}"));
    assert_eq!(strict.fingerprints(), permissive.fingerprints());
    assert_ne!(
        recite_compiler::BuildRequestIdentity::from_request(&strict),
        recite_compiler::BuildRequestIdentity::from_request(&permissive)
    );
    let fence = recite_compiler::BuildAuthorityFence::new(BuildAuthority::from_request(&strict));
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    let result = recite_compiler::BuildCoordinator::with_fence(fence)
        .run(
            permissive,
            &recite_compiler::BuildControl::new(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("identity refusal: {error}"));
    assert!(matches!(
        result.publish(),
        recite_compiler::PublishOutcome::Refused {
            reason: recite_compiler::PublishRefusal::RequestIdentityMismatch
        }
    ));
    assert_eq!(publisher.commit_calls, 0);
}

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
        ) -> Result<Vec<BuildCandidate>, recite_compiler::BuildFailure> {
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

#[test]
fn reducer_enforces_ready_and_terminal_identity_phases() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut lifecycle = BuildLifecycle::new();
    assert!(matches!(
        lifecycle.transition(BuildTransition::PublishStarted {
            prepared: PreparedPublishIdentity::for_request(&request, Vec::new())
        }),
        Err(BuildTransitionError::Invalid { .. })
    ));
    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start: {error}"));
    lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
        })
        .unwrap_or_else(|error| panic!("check: {error}"));
    assert!(
        matches!(
            lifecycle.transition(BuildTransition::CheckFailed {
                result: run(
                    request.clone(),
                    &recite_compiler::BuildControl::new(),
                    &mut FakeEngine::new([]),
                    &mut FakePublisher::new()
                )
            }),
            Err(BuildTransitionError::Invalid { .. })
        ),
        "check failure is legal only from checking"
    );
    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: Vec::new(),
        })
        .unwrap_or_else(|error| panic!("build: {error}"));
    assert!(matches!(lifecycle.state(), BuildState::Ready { .. }));
    assert!(matches!(
        lifecycle.transition(BuildTransition::BuildCompleted {
            candidates: Vec::new()
        }),
        Err(BuildTransitionError::Invalid { .. })
    ));
}

#[test]
fn duration_is_nonsemantic_and_results_keep_request_identity() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let result_a = run(
        request.clone(),
        &recite_compiler::BuildControl::new(),
        &mut FakeEngine::new([candidate("a.recitec", b"a")]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(1)));
    let result_b = run(
        request,
        &recite_compiler::BuildControl::new(),
        &mut FakeEngine::new([candidate("a.recitec", b"a")]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(2)));
    assert!(result_a.semantic_eq(&result_b));
    assert_eq!(result_a, result_b);
    assert_eq!(result_a.restart_guidance(), RestartGuidance::NotApplicable);
    assert_eq!(result_a.fingerprints(), result_b.fingerprints());
}

#[test]
fn freshness_stale_assessment_is_rebuildable() {
    struct StaleCheck;
    impl recite_compiler::BuildEngine for StaleCheck {
        fn check(
            &mut self,
            request: &BuildRequest,
            _: &recite_compiler::BuildControl,
        ) -> recite_compiler::BuildCheck {
            recite_compiler::BuildCheck::new(
                request,
                Vec::new(),
                recite_compiler::FreshnessAssessment::stale(
                    request.fingerprints().clone(),
                    vec![recite_compiler::StaleReason::Fingerprints],
                ),
            )
        }
        fn build(
            &mut self,
            _: &BuildRequest,
            _: &recite_compiler::BuildControl,
        ) -> Result<Vec<recite_compiler::BuildCandidate>, recite_compiler::BuildFailure> {
            Ok(vec![candidate("a.recitec", b"rebuilt")])
        }
    }
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut publisher = FakePublisher::new();
    let mut engine = StaleCheck;
    let result = run(
        request,
        &recite_compiler::BuildControl::new(),
        &mut engine,
        &mut publisher,
    );
    assert_eq!(
        result.status(),
        recite_compiler::BuildTerminalStatus::Succeeded
    );
    assert_eq!(
        result.freshness().status(),
        recite_compiler::FreshnessStatus::Stale
    );
    assert_eq!(
        publisher.published.get("a.recitec"),
        Some(&b"rebuilt".to_vec())
    );
}
