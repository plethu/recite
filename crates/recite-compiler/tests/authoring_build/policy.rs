use std::time::Duration;

use super::support::*;
use recite_compiler::{
    BuildInput, BuildInputAuthority, BuildInputKind, BuildInputPolicy, BuildLifecycle,
    BuildRequest, BuildState, BuildTelemetry, BuildTransition, BuildTransitionError,
    RestartGuidance,
};

#[test]
fn request_rejects_implicit_overlay_and_sorts_effective_inputs() {
    let overlay = BuildInput::overlay_source(key("dialogue/z.recite"), ":: overlay\n");
    let error = BuildRequest::new(
        recite_compiler::BuildGeneration::initial(),
        recite_compiler::SnapshotGeneration::initial(),
        [overlay],
    )
    .expect_err("saved-only request rejects an overlay");
    assert!(matches!(
        error,
        recite_compiler::BuildRequestError::OverlayNotAllowed { .. }
    ));

    let saved = BuildInput::saved_source(key("dialogue/z.recite"), ":: saved\n");
    let overlay = BuildInput::overlay_source(key("dialogue/z.recite"), ":: overlay\n");
    let request = BuildRequest::new_with_policy(
        recite_compiler::BuildGeneration::new(1),
        recite_compiler::SnapshotGeneration::new(1),
        [
            saved,
            overlay,
            BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n"),
        ],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("explicit overlay request is valid: {error}"));
    assert_eq!(request.inputs().len(), 2);
    assert_eq!(request.inputs()[0].key().as_str(), "dialogue/a.recite");
    assert_eq!(request.inputs()[1].key().as_str(), "dialogue/z.recite");
    assert_eq!(
        request.inputs()[1].authority(),
        recite_compiler::BuildInputAuthority::Overlay
    );
    assert_eq!(
        request
            .affected_inputs()
            .iter()
            .map(|input| input.input().key().as_str())
            .collect::<Vec<_>>(),
        ["dialogue/a.recite", "dialogue/z.recite"]
    );
}

#[test]
fn identical_payloads_are_order_invariant_for_requests_and_results() {
    let request_left = make_request(
        2,
        [
            BuildInput::saved_source(key("dialogue/z.recite"), ":: z\n"),
            BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n"),
        ],
    );
    let request_right = make_request(
        2,
        [
            BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n"),
            BuildInput::saved_source(key("dialogue/z.recite"), ":: z\n"),
        ],
    );
    assert_eq!(request_left, request_right);
    assert_eq!(request_left.fingerprints(), request_right.fingerprints());
    assert_eq!(request_left.inputs()[0].content(), ":: a\n");

    let mut engine_left =
        FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut publisher_left = FakePublisher::new();
    let mut engine_right =
        FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut publisher_right = FakePublisher::new();
    let control_left = recite_compiler::BuildControl::new();
    let control_right = recite_compiler::BuildControl::new();
    let result_left = run(
        request_left,
        &control_left,
        &mut engine_left,
        &mut publisher_left,
    );
    let result_right = run(
        request_right,
        &control_right,
        &mut engine_right,
        &mut publisher_right,
    );
    assert!(result_left.semantic_eq(&result_right));
    assert_eq!(result_left.candidates(), result_right.candidates());
}

#[test]
fn schema_freshness_uses_the_parsed_canonical_model() {
    let missing_model = BuildRequest::new(
        recite_compiler::BuildGeneration::new(2),
        recite_compiler::SnapshotGeneration::new(2),
        [BuildInput::new(
            key("schema/project.toml"),
            BuildInputKind::Schema,
            BuildInputAuthority::Saved,
            "schema_version = 1\n",
        )],
    )
    .expect_err("raw schema text cannot provide a semantic schema fingerprint");
    assert!(matches!(
        missing_model,
        recite_compiler::BuildRequestError::SchemaModelRequired { .. }
    ));

    let model = recite_core::ProjectSchema::empty_v1();
    let request_left = BuildRequest::new(
        recite_compiler::BuildGeneration::new(3),
        recite_compiler::SnapshotGeneration::new(3),
        [BuildInput::schema(
            key("schema/project.toml"),
            BuildInputAuthority::Saved,
            "schema_version = 1\n",
            model.clone(),
        )],
    )
    .unwrap_or_else(|error| panic!("schema request is valid: {error}"));
    let request_right = BuildRequest::new(
        recite_compiler::BuildGeneration::new(3),
        recite_compiler::SnapshotGeneration::new(3),
        [BuildInput::schema(
            key("schema/project.toml"),
            BuildInputAuthority::Saved,
            "# formatting and comments differ\nschema_version = 1\n",
            model,
        )],
    )
    .unwrap_or_else(|error| panic!("schema request is valid: {error}"));
    assert_ne!(request_left, request_right);
    assert_eq!(request_left.fingerprints(), request_right.fingerprints());
    assert_eq!(
        request_left.fingerprints().schema(),
        request_right.fingerprints().schema()
    );

    let mut engine = FakeEngine::new([candidate("dialogue.recitec", b"A")]);
    let mut publisher = FakePublisher::new();
    let control = recite_compiler::BuildControl::new();
    let authority = recite_compiler::BuildAuthority::from_request(&request_right);
    let result = recite_compiler::BuildCoordinator::new()
        .run(
            request_left,
            &control,
            &authority,
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("schema request transitions: {error}"));
    assert_eq!(
        result.status(),
        recite_compiler::BuildTerminalStatus::Succeeded
    );
    assert_eq!(publisher.commit_calls, 1);
}

#[test]
fn lifecycle_rejects_illegal_events_and_accepts_terminal_result() {
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let mut lifecycle = BuildLifecycle::new();
    let error = lifecycle
        .transition(BuildTransition::PublishStarted)
        .expect_err("publishing cannot begin while idle");
    assert!(matches!(error, BuildTransitionError::Invalid { .. }));
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    let control = recite_compiler::BuildControl::new();
    let result = run(request.clone(), &control, &mut engine, &mut publisher);
    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start is legal: {error}"));
    lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
        })
        .unwrap_or_else(|error| panic!("check is legal: {error}"));
    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: result.candidates().to_vec(),
        })
        .unwrap_or_else(|error| panic!("build is legal: {error}"));
    lifecycle
        .transition(BuildTransition::PublishStarted)
        .unwrap_or_else(|error| panic!("publish is legal: {error}"));
    lifecycle
        .transition(BuildTransition::PublishCompleted { result })
        .unwrap_or_else(|error| panic!("completion is legal: {error}"));
    assert!(matches!(lifecycle.state(), BuildState::Succeeded { .. }));
}

#[test]
fn duration_does_not_change_semantic_result_equality() {
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let mut engine_a = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher_a = FakePublisher::new();
    let control_a = recite_compiler::BuildControl::new();
    let result_a = run(request.clone(), &control_a, &mut engine_a, &mut publisher_a)
        .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(1)));
    let mut engine_b = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher_b = FakePublisher::new();
    let control_b = recite_compiler::BuildControl::new();
    let result_b = run(request, &control_b, &mut engine_b, &mut publisher_b)
        .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(2)));
    assert!(result_a.semantic_eq(&result_b));
    assert_eq!(result_a, result_b);
    assert_eq!(result_a.restart_guidance(), RestartGuidance::NotApplicable);
    assert!(result_a.telemetry().duration() == Some(Duration::from_millis(1)));
}

#[test]
fn result_candidates_and_published_targets_are_sorted() {
    let request = make_request(
        2,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let mut engine = FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    let control = recite_compiler::BuildControl::new();
    let result = run(request, &control, &mut engine, &mut publisher);
    assert_eq!(
        result
            .candidates()
            .iter()
            .map(|candidate| candidate.target().as_str())
            .collect::<Vec<_>>(),
        ["a.recitec", "z.recitec"]
    );
    assert_eq!(
        result.publish(),
        &recite_compiler::PublishOutcome::Published {
            targets: vec![target("a.recitec"), target("z.recitec")]
        }
    );
}
