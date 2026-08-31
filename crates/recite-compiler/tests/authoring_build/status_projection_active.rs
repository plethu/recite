use super::support::*;
use recite_compiler::{
    BuildInput, BuildLifecycle, BuildStatusProjection, BuildTransition, PreparedPublishIdentity,
};

#[test]
fn active_post_check_phases_retain_diagnostics_and_freshness() {
    let request = make_request(8, [BuildInput::saved_source(key("warning.recite"), "x")]);
    let diagnostic = recite_core::Diagnostic::new(
        recite_core::DiagnosticCode::new_static("RECITE_VALIDATE001"),
        recite_core::DiagnosticSeverity::Warning,
        "non-fatal test warning",
        recite_core::SourceSpan::point(
            "warning.recite",
            recite_core::SourcePosition::new(1, 1)
                .unwrap_or_else(|error| panic!("test position: {error}")),
        ),
    );
    let freshness = recite_compiler::FreshnessAssessment::stale(
        request.fingerprints().clone(),
        vec![recite_compiler::StaleReason::Fingerprints],
    );
    let candidates = vec![candidate("warning.recitec", b"compiled")];
    let mut lifecycle = BuildLifecycle::new();
    lifecycle
        .transition(BuildTransition::Start {
            request: request.clone(),
        })
        .unwrap_or_else(|error| panic!("start: {error}"));
    lifecycle
        .transition(BuildTransition::CheckPassed {
            freshness: freshness.clone(),
            diagnostics: vec![diagnostic.clone()],
        })
        .unwrap_or_else(|error| panic!("check: {error}"));
    let building = BuildStatusProjection::from_state(lifecycle.state());
    assert_eq!(building.diagnostics(), std::slice::from_ref(&diagnostic));
    assert_eq!(building.freshness(), Some(&freshness));

    lifecycle
        .transition(BuildTransition::BuildCompleted {
            candidates: candidates.clone(),
        })
        .unwrap_or_else(|error| panic!("build: {error}"));
    let ready = BuildStatusProjection::from_state(lifecycle.state());
    assert_eq!(ready.diagnostics(), std::slice::from_ref(&diagnostic));
    assert_eq!(ready.freshness(), Some(&freshness));

    lifecycle
        .transition(BuildTransition::PublishStarted {
            prepared: PreparedPublishIdentity::for_request(&request, candidates),
        })
        .unwrap_or_else(|error| panic!("publish start: {error}"));
    let publishing = BuildStatusProjection::from_state(lifecycle.state());
    assert_eq!(publishing.diagnostics(), &[diagnostic]);
    assert_eq!(publishing.freshness(), Some(&freshness));
}
