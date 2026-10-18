use super::*;
use recite_core::SourcePosition;

#[test]
fn remembered_duplicate_span_respects_project_completeness_and_source_order() {
    let spans = [
        ("a.recite", 1),
        ("b.recite", 2),
        ("a.recite", 3),
        ("a.recite", 4),
    ]
    .map(|(file, line)| {
        SourceSpan::new(file, SourcePosition::new(line, 1).expect("position"), None)
    });
    let mut complete = Validator::for_phase([], None, true, ValidationPhase::Project);
    assert_eq!(complete.record_localisable_id("shared", &spans[0]), None);
    for span in &spans[1..] {
        assert_eq!(
            complete.record_localisable_id("shared", span),
            Some(&spans[0])
        );
    }

    let mut incomplete = Validator::for_phase([], None, false, ValidationPhase::Project);
    assert_eq!(incomplete.record_localisable_id("shared", &spans[0]), None);
    assert_eq!(incomplete.record_localisable_id("shared", &spans[1]), None);
    assert_eq!(incomplete.record_localisable_id("shared", &spans[2]), None);
    assert_eq!(
        incomplete.record_localisable_id("shared", &spans[3]),
        Some(&spans[2])
    );
    assert_eq!(
        incomplete.record_localisable_id("shared", &spans[2]),
        Some(&spans[2])
    );
}
