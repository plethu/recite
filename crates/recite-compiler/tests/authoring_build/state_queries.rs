use recite_compiler::authoring::{
    BuildCandidate, BuildControl, BuildEventKind, BuildGeneration, BuildInput, BuildLifecycle,
    BuildPhase, BuildRequest, BuildState, BuildTerminalStatus, BuildTransition,
    BuildTransitionError, PreparedPublishIdentity,
};

use super::support::*;

#[test]
fn state_queries_expose_the_active_request_and_only_complete_results() {
    let request = make_request(3, [BuildInput::saved_source(key("main.recite"), "source")]);
    let candidates = vec![candidate("main.recitec", b"compiled")];
    let mut lifecycle = BuildLifecycle::new();
    assert_eq!(lifecycle.state(), &BuildState::Idle);
    assert_eq!(lifecycle.state().request(), None);
    assert_eq!(lifecycle.state().generation(), None);
    assert!(lifecycle.state().candidates().is_empty());
    assert_eq!(lifecycle.state().result(), None);
    assert!(!lifecycle.state().is_terminal());

    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .expect("start accepted");
    assert_active(lifecycle.state(), &request, &[]);
    lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
            diagnostics: Vec::new(),
        })
        .expect("check accepted");
    assert_active(lifecycle.state(), &request, &[]);
    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: candidates.clone(),
        })
        .expect("candidates accepted");
    assert_active(lifecycle.state(), &request, &candidates);
    lifecycle
        .transition(BuildTransition::PublishStarted {
            prepared: PreparedPublishIdentity::for_request(&request, candidates.clone()),
        })
        .expect("publish accepted");
    assert_active(lifecycle.state(), &request, &candidates);

    let result = run(
        request.clone(),
        &BuildControl::new(),
        &mut FakeEngine::new(candidates.clone()),
        &mut FakePublisher::new(),
    );
    assert_eq!(result.status(), BuildTerminalStatus::Succeeded);
    lifecycle
        .transition(BuildTransition::PublishCompleted {
            result: result.clone(),
        })
        .expect("completion accepted");
    assert!(lifecycle.state().is_terminal());
    assert_eq!(lifecycle.state().request(), None);
    assert_eq!(
        lifecycle.state().generation(),
        Some(BuildGeneration::new(3))
    );
    assert_eq!(lifecycle.state().candidates(), candidates);
    assert_eq!(lifecycle.state().result(), Some(&result));
}

#[test]
fn invalid_build_transitions_report_the_phase_and_event_without_changing_state() {
    let request = make_request(4, [BuildInput::saved_source(key("main.recite"), "source")]);
    let mut lifecycle = BuildLifecycle::new();
    let error = lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
            diagnostics: Vec::new(),
        })
        .expect_err("idle cannot accept a check result");
    assert_eq!(
        error,
        BuildTransitionError::Invalid {
            state: BuildPhase::Idle,
            event: BuildEventKind::CheckPassed,
        }
    );
    assert_eq!(
        error.to_string(),
        "cannot apply check-passed while build is idle"
    );
    assert_eq!(lifecycle.state(), &BuildState::Idle);

    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .expect("start accepted");
    let before = lifecycle.state().clone();
    let error = lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: vec![candidate("main.recitec", b"compiled")],
        })
        .expect_err("unchecked build cannot accept compiled candidates");
    assert_eq!(
        error,
        BuildTransitionError::Invalid {
            state: BuildPhase::Checking,
            event: BuildEventKind::BuildCompleted,
        }
    );
    assert_eq!(
        error.to_string(),
        "cannot apply build-completed while build is checking"
    );
    assert_eq!(lifecycle.state(), &before);
}

fn assert_active(state: &BuildState, request: &BuildRequest, candidates: &[BuildCandidate]) {
    assert_eq!(state.request(), Some(request));
    assert_eq!(state.generation(), Some(BuildGeneration::new(3)));
    assert_eq!(state.candidates(), candidates);
    assert_eq!(state.result(), None);
    assert!(!state.is_terminal());
}
