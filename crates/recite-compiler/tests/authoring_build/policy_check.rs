use super::support::*;
use recite_compiler::{
    BuildInput, BuildLifecycle, BuildState, BuildTransition, BuildTransitionError,
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
