use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildAuthorityFence, BuildCandidate, BuildCheck, BuildControl, BuildEngine,
    BuildFailure, BuildGeneration, BuildInput, BuildRequest, BuildResultFailure,
    BuildTerminalStatus, PublishOutcome, RecoveryNeeded,
};

#[test]
fn cancellation_at_each_checkpoint_never_calls_commit_and_aborts_staging() {
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

    let during_check = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    engine.cancellation = EngineCancellation::DuringCheck;
    let mut publisher = FakePublisher::new();
    assert_eq!(
        run(request.clone(), &during_check, &mut engine, &mut publisher).status(),
        BuildTerminalStatus::Cancelled
    );
    let during_build = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a")]);
    engine.cancellation = EngineCancellation::DuringBuild;
    let mut publisher = FakePublisher::new();
    assert_eq!(
        run(request.clone(), &during_build, &mut engine, &mut publisher).status(),
        BuildTerminalStatus::Cancelled
    );
    let between = BuildControl::new();
    let mut engine = FakeEngine::new([candidate("a.recitec", b"a"), candidate("b.recitec", b"b")]);
    let mut publisher = FakePublisher::new();
    publisher.cancel_after_prepare = Some(1);
    assert_eq!(
        run(request, &between, &mut engine, &mut publisher).status(),
        BuildTerminalStatus::Cancelled
    );
    assert_eq!(publisher.commit_calls, 0);
    assert!(publisher.staged.is_empty());
    assert_eq!(publisher.abort_calls, 1);
}

#[test]
fn supersession_dominates_cancellation_and_engine_failure() {
    struct FailingEngine;
    impl BuildEngine for FailingEngine {
        fn check(&mut self, request: &BuildRequest, _: &BuildControl) -> BuildCheck {
            BuildCheck::passed(request)
        }
        fn build(
            &mut self,
            _: &BuildRequest,
            control: &BuildControl,
        ) -> Result<Vec<BuildCandidate>, BuildFailure> {
            control.cancel();
            control.supersede(BuildGeneration::new(2));
            Err(BuildFailure::Engine {
                reason: recite_compiler::BuildFailureReason::Host,
            })
        }
    }
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let control = BuildControl::new();
    let mut engine = FailingEngine;
    let mut publisher = FakePublisher::new();
    let result = run(request, &control, &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Superseded);
    assert!(result.failure().is_none());
    assert_eq!(publisher.commit_calls, 0);
}

#[test]
fn stale_fence_and_invalid_partial_are_structured_failures() {
    let request = make_request(4, [BuildInput::saved_source(key("a.recite"), "a")]);
    let changed = make_request(4, [BuildInput::saved_source(key("a.recite"), "changed")]);
    let fence = BuildAuthorityFence::new(BuildAuthority::from_request(&changed));
    let mut engine = FakeEngine::new([candidate("a.recitec", b"A")]);
    let mut publisher = FakePublisher::new();
    let stale = recite_compiler::BuildCoordinator::with_fence(fence)
        .run(
            request.clone(),
            &BuildControl::new(),
            &mut engine,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("valid stale transition: {error}"));
    assert_eq!(stale.status(), BuildTerminalStatus::Stale);
    assert_eq!(publisher.commit_calls, 0);
    assert!(matches!(stale.publish(), PublishOutcome::Refused { .. }));

    let mut engine = FakeEngine::new([candidate("a.recitec", b"a"), candidate("b.recitec", b"b")]);
    let mut publisher = FakePublisher::new();
    publisher.commit_outcome = Some(PublishOutcome::Partial {
        committed: vec![target("a.recitec")],
        failed: target("a.recitec"),
        remaining: vec![target("b.recitec")],
        recovery: RecoveryNeeded::for_targets(vec![target("a.recitec")]),
    });
    let invalid = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert_eq!(invalid.status(), BuildTerminalStatus::Failed);
    assert!(matches!(
        invalid.failure(),
        Some(BuildResultFailure::InvalidPublication(_))
    ));
    assert!(matches!(
        invalid.publish(),
        PublishOutcome::Indeterminate { attempted, recovery }
            if attempted == &[target("a.recitec"), target("b.recitec")]
                && recovery.targets() == [target("a.recitec"), target("b.recitec")]
    ));
    assert_eq!(publisher.published.get("a.recitec"), Some(&b"a".to_vec()));
}

#[test]
fn preparation_failure_preserves_prior_outputs_and_reason() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut publisher = FakePublisher::new();
    publisher
        .published
        .insert("a.recitec".into(), b"prior".to_vec());
    publisher.fail_target = Some(target("b.recitec"));
    let mut engine = FakeEngine::new([
        candidate("a.recitec", b"new-a"),
        candidate("b.recitec", b"new-b"),
    ]);
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Failed);
    assert_eq!(publisher.commit_calls, 0);
    assert_eq!(
        publisher.published.get("a.recitec"),
        Some(&b"prior".to_vec())
    );
    let expected = target("b.recitec");
    assert!(
        matches!(result.failure(), Some(BuildResultFailure::Preparation { target, .. }) if target == &expected)
    );
}

#[test]
fn partial_commit_mutates_only_committed_bytes_and_reports_recovery() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
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
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Failed);
    assert_eq!(publisher.published.get("a.recitec"), Some(&b"a".to_vec()));
    assert!(!publisher.published.contains_key("b.recitec"));
    assert!(matches!(result.publish(), PublishOutcome::Partial { .. }));
}

#[test]
fn failure_detail_and_order_are_deterministic() {
    let request = make_request(2, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut engine = FakeEngine::new([candidate("z.recitec", b"z"), candidate("a.recitec", b"a")]);
    let mut publisher = FakePublisher::new();
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert_eq!(
        result
            .candidates()
            .iter()
            .map(|candidate| candidate.target().as_str())
            .collect::<Vec<_>>(),
        ["a.recitec", "z.recitec"]
    );
    assert!(
        matches!(result.publish(), PublishOutcome::Published { targets } if targets == &[target("a.recitec"), target("z.recitec")])
    );
}

#[test]
fn empty_candidates_complete_without_preparing_or_publishing() {
    let request = make_request(3, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut engine = FakeEngine::new([]);
    let mut publisher = FakePublisher::new();
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert_eq!(result.status(), BuildTerminalStatus::Succeeded);
    assert_eq!(
        result.publish(),
        &PublishOutcome::NotAttempted {
            reason: recite_compiler::PublishNotAttemptedReason::NoCandidates
        }
    );
    assert_eq!(publisher.prepare_calls, 0);
    assert_eq!(publisher.commit_calls, 0);
}

#[test]
fn check_identity_and_duplicate_targets_are_preserved_as_typed_failures() {
    struct MismatchedCheck {
        other: BuildRequest,
    }
    impl BuildEngine for MismatchedCheck {
        fn check(&mut self, _: &BuildRequest, _: &BuildControl) -> BuildCheck {
            BuildCheck::passed(&self.other)
        }
        fn build(
            &mut self,
            _: &BuildRequest,
            _: &BuildControl,
        ) -> Result<Vec<BuildCandidate>, BuildFailure> {
            Ok(Vec::new())
        }
    }
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let other = make_request(1, [BuildInput::saved_source(key("a.recite"), "changed")]);
    let mut engine = MismatchedCheck { other };
    let mut publisher = FakePublisher::new();
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    assert!(matches!(
        result.failure(),
        Some(BuildResultFailure::Check(_))
    ));
    assert_eq!(publisher.commit_calls, 0);

    let request = make_request(2, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut engine = FakeEngine::new([
        candidate("same.recitec", b"a"),
        candidate("same.recitec", b"b"),
    ]);
    let mut publisher = FakePublisher::new();
    let result = run(request, &BuildControl::new(), &mut engine, &mut publisher);
    let expected = target("same.recitec");
    assert!(
        matches!(result.failure(), Some(BuildResultFailure::DuplicateTarget { target }) if target == &expected)
    );
    assert_eq!(publisher.commit_calls, 0);
}
