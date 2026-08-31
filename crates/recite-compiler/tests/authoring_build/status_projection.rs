use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildCheck, BuildControl, BuildEngine, BuildFailure, BuildGeneration,
    BuildInput, BuildInputAuthority, BuildInputPolicy, BuildLifecycle, BuildRequest,
    BuildStatusProjection, BuildTerminalStatus, BuildTransition, PreparedPublishIdentity,
    PublishOutcome, RecoveryNeeded,
};

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
    );
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
