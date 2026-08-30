use super::support::*;

use recite_compiler::{
    BuildAuthority, BuildCandidate, BuildCheck, BuildControl, BuildEngine, BuildFailure,
    BuildGeneration, BuildInput, BuildRequest, BuildTerminalStatus, PublishNotAttemptedReason,
    PublishOutcome, RecoveryNeeded,
};

#[test]
fn cancellation_at_each_checkpoint_never_calls_commit() {
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let before = BuildControl::new();
    before.cancel();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    let result = run(request.clone(), &before, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(engine.check_calls, 0);
    assert_eq!(publisher.commit_calls, 0);
    assert!(matches!(
        result.publish(),
        PublishOutcome::NotAttempted {
            reason: PublishNotAttemptedReason::Cancelled
        }
    ));

    let during_check = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    engine.cancellation = EngineCancellation::DuringCheck;
    let mut publisher = FakePublisher::new();
    let result = run(request.clone(), &during_check, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(publisher.commit_calls, 0);

    let during_build = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    engine.cancellation = EngineCancellation::DuringBuild;
    let mut publisher = FakePublisher::new();
    let result = run(request.clone(), &during_build, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(publisher.commit_calls, 0);

    let between_candidates = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a"), candidate("b.recitec", b"b")]);
    let mut publisher = FakePublisher::new();
    publisher.cancel_after_prepare = Some(1);
    let result = run(request, &between_candidates, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(publisher.prepare_calls, 1);
    assert_eq!(publisher.commit_calls, 0);

    let before_publish = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    publisher.cancel_after_prepare = Some(1);
    let result = run(
        make_request(
            3,
            [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
        ),
        &before_publish,
        &mut engine,
        &mut publisher,
    );
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(publisher.prepare_calls, 1);
    assert_eq!(publisher.commit_calls, 0);
}

#[test]
fn supersession_wins_user_cancellation_and_only_newer_bytes_publish() {
    let request_a = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let request_b = make_request(
        2,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: b\n")],
    );
    let mut publisher = FakePublisher::new();
    let control_a = BuildControl::new();
    let mut engine_a = FakeEngine::new([candidate("dialogue.recitec", b"A")]);
    engine_a.cancellation = EngineCancellation::SupersedeDuringBuild(request_b.generation());
    let result_a = run(request_a, &control_a, &mut engine_a, &mut publisher);
    assert_eq!(result_a.status(), BuildTerminalStatus::Superseded);
    assert_eq!(publisher.commit_calls, 0);

    let control_b = BuildControl::new();
    let mut engine_b = FakeEngine::new([candidate("dialogue.recitec", b"B")]);
    let result_b = run(request_b, &control_b, &mut engine_b, &mut publisher);
    assert_eq!(result_b.status(), BuildTerminalStatus::Succeeded);
    assert_eq!(publisher.commit_calls, 1);
    assert_eq!(
        publisher.published.get("dialogue.recitec"),
        Some(&b"B".to_vec())
    );
}

#[test]
fn stale_generation_or_fingerprint_refuses_publish() {
    let request = make_request(
        4,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let control = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"A")]);
    let mut publisher = FakePublisher::new();
    let changed = BuildAuthority::new(
        BuildGeneration::new(5),
        request.snapshot_generation(),
        request.fingerprints().clone(),
    );
    let mut current = Some(changed);
    let result = recite_compiler::BuildCoordinator::new()
        .run_with_authority(
            request.clone(),
            &control,
            || {
                current
                    .take()
                    .unwrap_or_else(|| BuildAuthority::from_request(&request))
            },
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("test coordinator transition is valid: {error}"));
    assert_eq!(result.status(), BuildTerminalStatus::Stale);
    assert_eq!(publisher.commit_calls, 0);
    assert!(matches!(
        result.publish(),
        PublishOutcome::Refused {
            reason: recite_compiler::PublishRefusal::StaleBuildGeneration
        }
    ));

    let request_six = make_request(
        6,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let changed = BuildAuthority::new(
        request_six.generation(),
        request_six.snapshot_generation(),
        make_request(
            6,
            [BuildInput::saved_source(
                key("dialogue/a.recite"),
                ":: changed\n",
            )],
        )
        .fingerprints()
        .clone(),
    );
    let control = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"A")]);
    let mut publisher = FakePublisher::new();
    let result = recite_compiler::BuildCoordinator::new()
        .run_with_authority(
            request_six.clone(),
            &control,
            || changed.clone(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("test coordinator transition is valid: {error}"));
    assert_eq!(result.status(), BuildTerminalStatus::Stale);
    assert_eq!(publisher.commit_calls, 0);
    assert!(matches!(
        result.publish(),
        PublishOutcome::Refused {
            reason: recite_compiler::PublishRefusal::StaleFingerprints
        }
    ));
}

#[test]
fn preparation_failure_does_not_replace_prior_outputs() {
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let mut publisher = FakePublisher::new();
    publisher
        .published
        .insert("a.recitec".to_owned(), b"prior".to_vec());
    publisher.fail_target = Some(target("b.recitec"));
    let mut engine = FakeEngine::new([
        candidate("a.recitec", b"new-a"),
        candidate("b.recitec", b"new-b"),
    ]);
    let control = BuildControl::new();
    let result = run(request, &control, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Failed);
    assert_eq!(publisher.commit_calls, 0);
    assert_eq!(
        publisher.published.get("a.recitec"),
        Some(&b"prior".to_vec())
    );
    assert!(matches!(
        result.publish(),
        PublishOutcome::NotAttempted {
            reason: PublishNotAttemptedReason::PreparationFailed
        }
    ));
}

#[test]
fn partial_commit_reports_exact_targets_and_recovery() {
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let mut engine = FakeEngine::new([
        candidate("a.recitec", b"a"),
        candidate("b.recitec", b"b"),
        candidate("c.recitec", b"c"),
    ]);
    let mut publisher = FakePublisher::new();
    publisher.commit_outcome = Some(PublishOutcome::Partial {
        committed: vec![target("a.recitec")],
        failed: target("b.recitec"),
        remaining: vec![target("c.recitec")],
        recovery: RecoveryNeeded::for_targets(vec![target("a.recitec"), target("b.recitec")]),
    });
    let control = BuildControl::new();
    let result = run(request, &control, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Failed);
    assert_eq!(publisher.prepare_calls, 3);
    assert_eq!(publisher.commit_calls, 1);
    assert!(
        matches!(result.publish(), PublishOutcome::Partial { committed, failed, remaining, recovery } if committed == &[target("a.recitec")] && failed == &target("b.recitec") && remaining == &[target("c.recitec")] && recovery.targets() == [target("a.recitec"), target("b.recitec")])
    );
}

fn error_diagnostic() -> recite_core::Diagnostic {
    let position = recite_core::SourcePosition::new(1, 1)
        .unwrap_or_else(|error| panic!("test position is valid: {error}"));
    recite_core::Diagnostic::new(
        recite_core::DiagnosticCode::new_static("RECITE_TEST001"),
        recite_core::DiagnosticSeverity::Error,
        "invalid test input",
        recite_core::SourceSpan::point("dialogue/test.recite", position),
    )
}

#[test]
fn failed_check_keeps_structured_diagnostics_and_no_build_or_publish() {
    struct FailingCheck;
    impl BuildEngine for FailingCheck {
        fn check(&mut self, request: &BuildRequest, _control: &BuildControl) -> BuildCheck {
            BuildCheck::new(vec![error_diagnostic()], freshness(request))
        }
        fn build(
            &mut self,
            _request: &BuildRequest,
            _control: &BuildControl,
        ) -> Result<Vec<BuildCandidate>, BuildFailure> {
            Err(BuildFailure::Engine {
                reason: recite_compiler::BuildFailureReason::Unknown,
            })
        }
    }
    let request = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let authority = BuildAuthority::from_request(&request);
    let control = BuildControl::new();
    let mut engine = FailingCheck;
    let mut publisher = FakePublisher::new();
    let result = recite_compiler::BuildCoordinator::new()
        .run(request, &control, &authority, &mut engine, &mut publisher)
        .unwrap_or_else(|error| panic!("test coordinator transition is valid: {error}"));
    assert_eq!(result.status(), BuildTerminalStatus::Failed);
    assert_eq!(result.diagnostics(), &[error_diagnostic()]);
    assert_eq!(publisher.commit_calls, 0);
}
