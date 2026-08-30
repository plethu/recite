use super::support::*;
use recite_compiler::{
    BuildGeneration, BuildInput, BuildInputAuthority, BuildInputKind, BuildInputPayload,
    BuildInputPolicy, BuildLifecycle, BuildRequest, BuildState, BuildTelemetry, BuildTransition,
    BuildTransitionError, PreparedPublish, RestartGuidance,
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
fn reducer_enforces_ready_and_terminal_identity_phases() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut lifecycle = BuildLifecycle::new();
    assert!(matches!(
        lifecycle.transition(BuildTransition::PublishStarted {
            prepared: PreparedPublish::new(&request, Vec::new()).identity()
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
fn freshness_stale_assessment_cannot_pass() {
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
            Ok(Vec::new())
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
        recite_compiler::BuildTerminalStatus::Failed
    );
    assert!(matches!(
        result.failure(),
        Some(recite_compiler::BuildResultFailure::Check(_))
    ));
}
