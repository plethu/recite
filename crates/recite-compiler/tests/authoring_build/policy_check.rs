use super::support::*;
use recite_compiler::{
    BuildCandidate, BuildCheck, BuildControl, BuildEngine, BuildFailure, BuildInput,
    BuildLifecycle, BuildRequest, BuildState, BuildTransition, BuildTransitionError,
    PreparedPublishIdentity,
};

#[test]
fn reducer_rejects_error_diagnostics_before_building() {
    let request = make_request(11, [BuildInput::saved_source(key("a.recite"), "a")]);
    let mut lifecycle = BuildLifecycle::new();
    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start: {error}"));
    let error = recite_core::Diagnostic::error(
        recite_core::DiagnosticCode::new_static("RECITE_VALIDATE001"),
        "invalid test content",
        recite_core::SourceSpan::point(
            "a.recite",
            recite_core::SourcePosition::new(1, 1)
                .unwrap_or_else(|position_error| panic!("test position: {position_error}")),
        ),
    );
    assert!(matches!(
        lifecycle.transition(BuildTransition::CheckPassed {
            freshness: freshness(&request),
            diagnostics: vec![error],
        }),
        Err(BuildTransitionError::CheckContainsErrors)
    ));
    assert!(matches!(lifecycle.state(), BuildState::Checking { .. }));
}

#[test]
fn terminal_reducer_rejects_contradictory_retained_check_metadata() {
    let request = make_request(12, [BuildInput::saved_source(key("a.recite"), "a")]);
    let candidates = vec![candidate("a.recitec", b"a")];
    let stale = recite_compiler::FreshnessAssessment::stale(
        request.fingerprints().clone(),
        vec![recite_compiler::StaleReason::Fingerprints],
    );
    let mut lifecycle = checked_lifecycle(&request, stale, vec![warning("a.recite")]);
    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: candidates.clone(),
        })
        .unwrap_or_else(|error| panic!("build: {error}"));
    let result = run(
        request.clone(),
        &BuildControl::new(),
        &mut FakeEngine::new(candidates.clone()),
        &mut FakePublisher::new(),
    );
    assert!(matches!(
        lifecycle.transition(BuildTransition::Failed { result }),
        Err(BuildTransitionError::ResultFreshnessMismatch)
    ));

    let mut lifecycle = checked_lifecycle(&request, freshness(&request), vec![warning("a.recite")]);
    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: candidates.clone(),
        })
        .unwrap_or_else(|error| panic!("build: {error}"));
    lifecycle
        .transition(BuildTransition::PublishStarted {
            prepared: PreparedPublishIdentity::for_request(&request, candidates.clone()),
        })
        .unwrap_or_else(|error| panic!("publish start: {error}"));
    let result = run(
        request,
        &BuildControl::new(),
        &mut FakeEngine::new(candidates),
        &mut FakePublisher::new(),
    );
    assert!(matches!(
        lifecycle.transition(BuildTransition::PublishCompleted { result }),
        Err(BuildTransitionError::ResultDiagnosticsMismatch)
    ));
}

#[test]
fn terminal_reducer_accepts_diagnostics_appended_after_check() {
    struct DiagnosticBuildFailure;
    impl BuildEngine for DiagnosticBuildFailure {
        fn check(&mut self, request: &BuildRequest, _: &BuildControl) -> BuildCheck {
            BuildCheck::new(request, vec![warning("a.recite")], freshness(request))
        }
        fn build(
            &mut self,
            _: &BuildRequest,
            _: &BuildControl,
        ) -> Result<Vec<BuildCandidate>, BuildFailure> {
            Err(BuildFailure::Diagnostics {
                diagnostics: vec![recite_core::Diagnostic::error(
                    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE002"),
                    "build diagnostic",
                    recite_core::SourceSpan::point(
                        "a.recite",
                        recite_core::SourcePosition::new(2, 1)
                            .unwrap_or_else(|error| panic!("test position: {error}")),
                    ),
                )],
            })
        }
    }
    let request = make_request(13, [BuildInput::saved_source(key("a.recite"), "a")]);
    let result = run(
        request.clone(),
        &BuildControl::new(),
        &mut DiagnosticBuildFailure,
        &mut FakePublisher::new(),
    );
    assert_eq!(
        result.diagnostics(),
        &[warning("a.recite"), error_diagnostic()]
    );
    let mut lifecycle = checked_lifecycle(&request, freshness(&request), vec![warning("a.recite")]);
    lifecycle
        .transition(BuildTransition::Failed { result })
        .unwrap_or_else(|error| panic!("terminal result: {error}"));
    assert!(matches!(lifecycle.state(), BuildState::Failed { .. }));
}

fn checked_lifecycle(
    request: &BuildRequest,
    freshness: recite_compiler::FreshnessAssessment,
    diagnostics: Vec<recite_core::Diagnostic>,
) -> BuildLifecycle {
    let mut lifecycle = BuildLifecycle::new();
    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start: {error}"));
    lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness,
            diagnostics,
        })
        .unwrap_or_else(|error| panic!("check: {error}"));
    lifecycle
}

fn error_diagnostic() -> recite_core::Diagnostic {
    recite_core::Diagnostic::error(
        recite_core::DiagnosticCode::new_static("RECITE_VALIDATE002"),
        "build diagnostic",
        recite_core::SourceSpan::point(
            "a.recite",
            recite_core::SourcePosition::new(2, 1)
                .unwrap_or_else(|error| panic!("test position: {error}")),
        ),
    )
}
