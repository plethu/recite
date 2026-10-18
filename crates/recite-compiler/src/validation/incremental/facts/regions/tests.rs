use super::super::{Identity, ProjectFacts};
use crate::validation::{ValidationInput, ValidationParticipation};
use recite_core::ast::ChoiceEcho;
use std::sync::Arc;

const SOURCE: &str = ":: a default\n> line@11111111111111111111\n  First.\n? reply@22222222222222222222 echo=line(11111111111111111111)\n  Reply.\n  -> a\n:: b\n-> END\n";

fn facts(source: &str) -> Arc<ProjectFacts> {
    let lowered = recite_parser::parse("a.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty());
    Arc::new(ProjectFacts::collect(ValidationInput::all_complete(
        &lowered.source_file,
    )))
}

fn join(fresh: &Arc<ProjectFacts>, previous: &Arc<ProjectFacts>) -> Arc<ProjectFacts> {
    ProjectFacts::join_regions(
        &fresh.path,
        fresh.participation,
        &[(fresh, &fresh.region_ranges())],
        Some(previous),
    )
}

#[test]
fn reference_only_revisions_share_exact_passages_and_keep_current_participation() {
    let previous = facts(SOURCE);
    let original = previous.as_ref().clone();
    let fresh = facts(&SOURCE.replace("  -> a", "  -> b"));
    let joined = join(&fresh, &previous);
    assert_eq!(joined, fresh);
    assert!(!Arc::ptr_eq(&joined, &previous));
    assert!(Arc::ptr_eq(&joined.passages, &previous.passages));
    assert!(Arc::ptr_eq(&joined.blocks, &previous.blocks));
    assert!(!Arc::ptr_eq(&joined.references, &previous.references));
    let mut incomplete = fresh.as_ref().clone();
    incomplete.participation = ValidationParticipation::all_incomplete();
    let incomplete = Arc::new(incomplete);
    let joined = join(&incomplete, &previous);
    assert_eq!(joined, incomplete);
    assert!(Arc::ptr_eq(&joined.passages, &previous.passages));
    assert_eq!(previous.as_ref(), &original);
}

#[test]
fn changed_identity_echo_frozen_state_and_coordinates_require_new_passages() {
    let previous = facts(SOURCE);
    let original = previous.as_ref().clone();
    let mut frozen = original.clone();
    Arc::make_mut(&mut frozen.passages)[0].frozen = false;
    let mut missing_id = original.clone();
    Arc::make_mut(&mut missing_id.passages)[0].identity = Identity::Line(None);
    let mut echo = original.clone();
    if let Identity::Choice(_, echo) = &mut Arc::make_mut(&mut echo.passages)[1].identity {
        *echo = ChoiceEcho::SelectedText;
    } else {
        panic!("choice fixture");
    }
    for fresh in [
        Arc::new(frozen),
        Arc::new(missing_id),
        Arc::new(echo),
        facts(&SOURCE.replace("11111111111111111111", "33333333333333333333")),
        facts(&SOURCE.replace("First.", "First.\n  More.")),
    ] {
        let joined = join(&fresh, &previous);
        assert_eq!(joined, fresh);
        assert!(!Arc::ptr_eq(&joined.passages, &previous.passages));
        assert_eq!(
            Arc::ptr_eq(&joined.blocks, &previous.blocks),
            fresh.blocks == previous.blocks
        );
        assert_eq!(
            Arc::ptr_eq(&joined.references, &previous.references),
            fresh.references == previous.references
        );
        assert_eq!(previous.as_ref(), &original);
    }
}

#[test]
fn relocating_a_shared_fact_vector_preserves_published_spans() {
    let previous = facts(SOURCE);
    let original = previous.as_ref().clone();
    let mut shifted = original.clone();
    let ranges = shifted.region_ranges();
    shifted.relocate_region(&ranges, |span| {
        span.start =
            recite_core::SourcePosition::new(span.start.line() + 1, span.start.column()).unwrap();
        span.end = span
            .end
            .map(|end| recite_core::SourcePosition::new(end.line() + 1, end.column()).unwrap());
    });
    assert_eq!(previous.as_ref(), &original);
    assert!(!Arc::ptr_eq(&shifted.passages, &previous.passages));
    assert_eq!(
        shifted.passages[0].span.start.line(),
        original.passages[0].span.start.line() + 1
    );
}
