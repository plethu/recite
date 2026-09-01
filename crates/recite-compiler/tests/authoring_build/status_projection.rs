use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildCancellation, BuildCheck, BuildControl, BuildEngine, BuildFailure,
    BuildGeneration, BuildInput, BuildInputAuthority, BuildInputPolicy, BuildLifecycle,
    BuildRequest, BuildStatusProjection, BuildTelemetry, BuildTerminalStatus, BuildTransition,
    FreshnessFinalization, PreparedPublishIdentity, PublishOutcome, RecoveryNeeded,
};
use std::time::Duration;

#[test]
fn projects_every_lifecycle_state_with_stable_fields() {
    let request = BuildRequest::new_with_policy(
        BuildGeneration::new(1),
        recite_compiler::SnapshotGeneration::new(9),
        [BuildInput::overlay_source(
            key("dialogue/start.recite"),
            "overlay",
        )],
        BuildInputPolicy::SavedAndOverlays,
    )
    .unwrap_or_else(|error| panic!("overlay request: {error}"));
    let candidates = vec![candidate("dialogue/start.recitec", b"compiled")];

    let idle = BuildLifecycle::new();
    assert_eq!(
        BuildStatusProjection::from_state(idle.state()).phase(),
        recite_compiler::BuildPhase::Idle
    );

    let mut checking = BuildLifecycle::new();
    checking
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start: {error}"));
    let checking_projection = BuildStatusProjection::from_state(checking.state());
    assert_eq!(
        checking_projection.phase(),
        recite_compiler::BuildPhase::Checking
    );
    assert_eq!(
        checking_projection.generation(),
        Some(BuildGeneration::new(1))
    );
    assert_eq!(
        checking_projection.snapshot_generation(),
        Some(recite_compiler::SnapshotGeneration::new(9))
    );
    assert_eq!(
        checking_projection
            .request_identity()
            .map(|identity| identity.policy()),
        Some(BuildInputPolicy::SavedAndOverlays)
    );
    assert_eq!(
        checking_projection
            .request_identity()
            .map(|identity| identity.fingerprints().inputs()[0].authority()),
        Some(BuildInputAuthority::Overlay)
    );
    assert_eq!(
        checking_projection.restart_guidance(),
        Some(recite_compiler::RestartGuidance::NotApplicable)
    );

    checking
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
            diagnostics: Vec::new(),
        })
        .unwrap_or_else(|error| panic!("check: {error}"));
    assert_eq!(
        BuildStatusProjection::from_state(checking.state()).phase(),
        recite_compiler::BuildPhase::Building
    );
    checking
        .transition(BuildTransition::BuildCompleted {
            candidates: candidates.clone(),
        })
        .unwrap_or_else(|error| panic!("build: {error}"));
    let ready_projection = BuildStatusProjection::from_state(checking.state());
    assert_eq!(ready_projection.phase(), recite_compiler::BuildPhase::Ready);
    assert_eq!(ready_projection.candidates(), candidates);

    let prepared = PreparedPublishIdentity::for_request(&request, candidates.clone());
    checking
        .transition(BuildTransition::PublishStarted { prepared })
        .unwrap_or_else(|error| panic!("publish start: {error}"));
    let publishing_projection = BuildStatusProjection::from_state(checking.state());
    assert_eq!(
        publishing_projection.phase(),
        recite_compiler::BuildPhase::Publishing
    );
    assert_eq!(publishing_projection.candidates(), candidates);

    let result = run(
        request.clone(),
        &recite_compiler::BuildControl::new(),
        &mut FakeEngine::new([candidate("dialogue/start.recitec", b"compiled")]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(23)));
    checking
        .transition(BuildTransition::PublishCompleted {
            result: result.clone(),
        })
        .unwrap_or_else(|error| panic!("publish complete: {error}"));
    let succeeded_projection = BuildStatusProjection::from_state(checking.state());
    assert_eq!(
        succeeded_projection.phase(),
        recite_compiler::BuildPhase::Succeeded
    );
    assert_eq!(
        succeeded_projection.terminal_status(),
        Some(BuildTerminalStatus::Succeeded)
    );
    assert_eq!(
        succeeded_projection.telemetry().duration(),
        Some(Duration::from_millis(23))
    );
    assert_eq!(
        succeeded_projection,
        BuildStatusProjection::from_result(&result)
    );

    let failed_request = make_request(2, [BuildInput::saved_source(key("failed.recite"), "x")]);
    let failed_result = run(
        failed_request.clone(),
        &recite_compiler::BuildControl::new(),
        &mut FakeEngine::new([
            candidate("failed.recitec", b"one"),
            candidate("failed.recitec", b"two"),
        ]),
        &mut FakePublisher::new(),
    );
    let mut failed = BuildLifecycle::new();
    failed
        .transition(BuildTransition::Start {
            request: failed_request.clone(),
        })
        .unwrap_or_else(|error| panic!("failed start: {error}"));
    failed
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&failed_request),
            diagnostics: Vec::new(),
        })
        .unwrap_or_else(|error| panic!("failed check: {error}"));
    failed
        .transition(BuildTransition::BuildCompleted {
            candidates: failed_result.candidates().to_vec(),
        })
        .unwrap_or_else(|error| panic!("failed build: {error}"));
    failed
        .transition(BuildTransition::Failed {
            result: failed_result.clone(),
        })
        .unwrap_or_else(|error| panic!("failed terminal: {error}"));
    assert_eq!(
        BuildStatusProjection::from_state(failed.state()).phase(),
        recite_compiler::BuildPhase::Failed
    );

    let stale_request = make_request(3, [BuildInput::saved_source(key("stale.recite"), "old")]);
    let current_request = make_request(3, [BuildInput::saved_source(key("stale.recite"), "new")]);
    let stale_result = BuildAuthority::from_request(&current_request);
    let stale_result = recite_compiler::BuildCoordinator::with_fence(
        recite_compiler::BuildAuthorityFence::new(stale_result),
    )
    .run(
        stale_request.clone(),
        &recite_compiler::BuildControl::new(),
        &mut FakeEngine::new([candidate("stale.recitec", b"stale")]),
        &mut FakePublisher::new(),
    )
    .unwrap_or_else(|error| panic!("stale run: {error}"));
    let mut stale = BuildLifecycle::new();
    stale
        .transition(BuildTransition::Start {
            request: stale_request.clone(),
        })
        .unwrap_or_else(|error| panic!("stale start: {error}"));
    stale
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&stale_request),
            diagnostics: Vec::new(),
        })
        .unwrap_or_else(|error| panic!("stale check: {error}"));
    stale
        .transition(BuildTransition::BuildCompleted {
            candidates: stale_result.candidates().to_vec(),
        })
        .unwrap_or_else(|error| panic!("stale build: {error}"));
    stale
        .transition(BuildTransition::Stale {
            result: stale_result.clone(),
        })
        .unwrap_or_else(|error| panic!("stale terminal: {error}"));
    assert_eq!(
        BuildStatusProjection::from_state(stale.state()).phase(),
        recite_compiler::BuildPhase::Stale
    );

    for (generation, status, control) in [
        (4, BuildTerminalStatus::Cancelled, cancelled_control()),
        (5, BuildTerminalStatus::Superseded, superseded_control()),
    ] {
        let request = make_request(
            generation,
            [BuildInput::saved_source(key("interrupted.recite"), "x")],
        );
        let result = run(
            request.clone(),
            &control,
            &mut FakeEngine::new([]),
            &mut FakePublisher::new(),
        );
        assert_eq!(result.status(), status);
        let expected_cancellation = match status {
            BuildTerminalStatus::Cancelled => Some(BuildCancellation::User),
            BuildTerminalStatus::Superseded => Some(BuildCancellation::Superseded {
                by: BuildGeneration::new(9),
            }),
            _ => unreachable!("only interrupted statuses are tested"),
        };
        assert_eq!(result.cancellation(), expected_cancellation);
        assert_eq!(
            BuildStatusProjection::from_result(&result).cancellation(),
            expected_cancellation
        );
        let mut lifecycle = BuildLifecycle::new();
        lifecycle
            .transition(BuildTransition::Start { request })
            .unwrap_or_else(|error| panic!("interrupted start: {error}"));
        let transition = match status {
            BuildTerminalStatus::Cancelled => BuildTransition::Cancelled { result },
            BuildTerminalStatus::Superseded => BuildTransition::Superseded { result },
            BuildTerminalStatus::Succeeded
            | BuildTerminalStatus::Failed
            | BuildTerminalStatus::Stale => unreachable!("only interrupted statuses are tested"),
            _ => unreachable!("unknown terminal status is not constructible here"),
        };
        lifecycle
            .transition(transition)
            .unwrap_or_else(|error| panic!("interrupted terminal: {error}"));
        assert_eq!(
            BuildStatusProjection::from_state(lifecycle.state()).terminal_status(),
            Some(status)
        );
        assert_eq!(
            BuildStatusProjection::from_state(lifecycle.state()).cancellation(),
            expected_cancellation
        );
    }
}

#[test]
fn projection_repeats_deterministically_and_retains_recovery_truth() {
    let request = make_request(6, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut engine = FakeEngine::new([
        candidate("z.recitec", b"z"),
        candidate("b.recitec", b"b"),
        candidate("a.recitec", b"a"),
    ]);
    let mut publisher = FakePublisher::new();
    publisher.commit_outcome = Some(PublishOutcome::Partial {
        committed: vec![target("a.recitec")],
        failed: target("b.recitec"),
        remaining: vec![target("z.recitec")],
        recovery: RecoveryNeeded::for_targets(vec![target("a.recitec")]),
    });
    let result = run(
        request,
        &recite_compiler::BuildControl::new(),
        &mut engine,
        &mut publisher,
    );
    let first = BuildStatusProjection::from_result(&result);
    let second = BuildStatusProjection::from_result(&result);
    assert_eq!(first, second);
    assert_eq!(first.candidates()[0].target().as_str(), "a.recitec");
    assert_eq!(first.candidates()[1].target().as_str(), "b.recitec");
    assert_eq!(first.candidates()[2].target().as_str(), "z.recitec");
    assert_eq!(first.publish(), Some(result.publish()));
    assert!(matches!(
        first.publish(),
        Some(PublishOutcome::Partial { recovery, .. })
            if recovery.targets() == [target("a.recitec")]
    ));
    assert!(first.failure().is_none());
}

#[test]
fn finalizing_post_publish_freshness_updates_shared_state_truthfully() {
    let request = make_request(20, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut coordinator = recite_compiler::BuildCoordinator::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    coordinator
        .run(
            request.clone(),
            &BuildControl::new(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("successful run: {error}"));
    let stale = coordinator
        .finalize_freshness(FreshnessFinalization::Stale {
            assessment: recite_compiler::FreshnessAssessment::stale(
                request.fingerprints().clone(),
                vec![recite_compiler::StaleReason::Fingerprints],
            ),
            diagnostics: vec![warning("a.recite")],
            recovery: Some(RecoveryNeeded::for_targets(vec![target("a.recitec")])),
        })
        .unwrap_or_else(|error| panic!("stale finalization: {error}"));
    assert_eq!(stale.status(), BuildTerminalStatus::Stale);
    assert!(matches!(stale.publish(), PublishOutcome::Published { .. }));
    let projection = BuildStatusProjection::from_state(coordinator.state());
    assert_eq!(projection.phase(), recite_compiler::BuildPhase::Stale);
    assert_eq!(
        projection.freshness().map(|value| value.status()),
        Some(recite_compiler::FreshnessStatus::Stale)
    );
    assert_eq!(
        projection.recovery().map(|value| value.targets()),
        Some([target("a.recitec")].as_slice())
    );
    assert_eq!(projection.diagnostics(), &[warning("a.recite")]);

    let request = make_request(21, [BuildInput::saved_source(key("b.recite"), "b")]);
    let mut engine = FakeEngine::new([candidate("b.recitec", b"b")]);
    let mut publisher = FakePublisher::new();
    coordinator
        .run(
            request.clone(),
            &BuildControl::new(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("second successful run: {error}"));
    let failed = coordinator
        .finalize_freshness(FreshnessFinalization::Indeterminate {
            assessment: recite_compiler::FreshnessAssessment::not_assessed(
                request.fingerprints().clone(),
            ),
            diagnostics: Vec::new(),
            recovery: None,
            reason: recite_compiler::FreshnessFailureReason::RecheckFailed,
        })
        .unwrap_or_else(|error| panic!("indeterminate finalization: {error}"));
    assert_eq!(failed.status(), BuildTerminalStatus::Failed);
    assert!(matches!(failed.publish(), PublishOutcome::Published { .. }));
    let projection = BuildStatusProjection::from_state(coordinator.state());
    assert_eq!(projection.phase(), recite_compiler::BuildPhase::Failed);
    assert_eq!(
        projection.freshness().map(|value| value.status()),
        Some(recite_compiler::FreshnessStatus::Unknown)
    );
    assert!(matches!(
        projection.failure(),
        Some(recite_compiler::BuildResultFailure::Freshness { .. })
    ));
}

#[test]
fn projection_retains_structured_diagnostics_and_freshness() {
    struct DiagnosticEngine;
    impl BuildEngine for DiagnosticEngine {
        fn check(&mut self, request: &BuildRequest, _: &BuildControl) -> BuildCheck {
            BuildCheck::new(
                request,
                vec![recite_core::Diagnostic::error(
                    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE001"),
                    "invalid test content",
                    recite_core::SourceSpan::point(
                        "dialogue/diagnostic.recite",
                        recite_core::SourcePosition::new(1, 1)
                            .unwrap_or_else(|error| panic!("test position: {error}")),
                    ),
                )],
                freshness(request),
            )
        }

        fn build(
            &mut self,
            _: &BuildRequest,
            _: &BuildControl,
        ) -> Result<Vec<recite_compiler::BuildCandidate>, BuildFailure> {
            unreachable!("a failed check never builds")
        }
    }

    let request = make_request(
        7,
        [BuildInput::saved_source(
            key("dialogue/diagnostic.recite"),
            "invalid",
        )],
    );
    let result = run(
        request,
        &BuildControl::new(),
        &mut DiagnosticEngine,
        &mut FakePublisher::new(),
    );
    let projection = BuildStatusProjection::from_result(&result);
    assert_eq!(projection.diagnostics(), result.diagnostics());
    assert_eq!(projection.failure(), result.failure());
    assert_eq!(projection.freshness(), Some(result.freshness()));
}

#[test]
fn projection_preserves_host_telemetry_without_making_it_semantic() {
    let request = make_request(8, [BuildInput::saved_source(key("timed.recite"), "x")]);
    let result = run(
        request,
        &BuildControl::new(),
        &mut FakeEngine::new([candidate("timed.recitec", b"compiled")]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(37)));

    let projection = BuildStatusProjection::from_result(&result);
    assert_eq!(
        projection.telemetry().duration(),
        Some(Duration::from_millis(37))
    );

    let other_duration = result
        .clone()
        .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(91)));
    assert_eq!(result, other_duration);
    let other_projection = BuildStatusProjection::from_result(&other_duration);
    assert_ne!(
        BuildStatusProjection::from_result(&result)
            .telemetry()
            .duration(),
        other_projection.telemetry().duration()
    );
    assert_eq!(projection, other_projection);
    assert!(projection.semantic_eq(&other_projection));
}

#[test]
fn projection_preserves_terminal_telemetry_for_failure_and_cancellation() {
    let failed = run(
        make_request(9, [BuildInput::saved_source(key("failed.recite"), "x")]),
        &BuildControl::new(),
        &mut FakeEngine::new([
            candidate("failed.recitec", b"first"),
            candidate("failed.recitec", b"second"),
        ]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(41)));
    assert_eq!(failed.status(), BuildTerminalStatus::Failed);
    assert_eq!(
        BuildStatusProjection::from_result(&failed)
            .telemetry()
            .duration(),
        Some(Duration::from_millis(41))
    );

    let control = cancelled_control();
    let cancelled = run(
        make_request(10, [BuildInput::saved_source(key("cancelled.recite"), "x")]),
        &control,
        &mut FakeEngine::new([]),
        &mut FakePublisher::new(),
    )
    .with_telemetry(BuildTelemetry::from_duration(Duration::from_millis(59)));
    assert_eq!(cancelled.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(
        BuildStatusProjection::from_result(&cancelled)
            .telemetry()
            .duration(),
        Some(Duration::from_millis(59))
    );
}

fn cancelled_control() -> recite_compiler::BuildControl {
    let control = recite_compiler::BuildControl::new();
    control.cancel();
    control
}

fn superseded_control() -> recite_compiler::BuildControl {
    let control = recite_compiler::BuildControl::new();
    control.supersede(BuildGeneration::new(9));
    control
}
