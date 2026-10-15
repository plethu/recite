use super::super::AuthoringSummary;
use std::sync::Arc;

fn summary(source: &str) -> Arc<AuthoringSummary> {
    let lowered = recite_parser::parse("a.recite", source).lower_source_file();
    Arc::new(AuthoringSummary::from_source_file(&lowered.source_file))
}

#[test]
fn reference_changes_share_only_identical_stable_id_records() {
    let source = ":: a default\n> a@11111111111111111111\n  One.\n-> b\n:: b\n> b@22222222222222222222\n  Two.\n-> END\n";
    let previous = summary(source);
    let original = previous.as_ref().clone();
    let reference_edit = summary(&source.replace("-> b", "-> a"));
    let joined = AuthoringSummary::join_regions(
        &[(&reference_edit, &reference_edit.region_ranges())],
        Some(&previous),
    );
    assert_eq!(joined, reference_edit);
    assert!(!Arc::ptr_eq(&joined, &previous));
    assert!(Arc::ptr_eq(&joined.stable_ids, &previous.stable_ids));
    for edited in [
        source.replace("11111111111111111111", "33333333333333333333"),
        source.replace("> a@", "? a@"),
        source.replace("One.", "One.\n  More."),
        source.replace(":: b", ":: c"),
        source.replace("> b@", "> bb@"),
    ] {
        let fresh = summary(&edited);
        let joined =
            AuthoringSummary::join_regions(&[(&fresh, &fresh.region_ranges())], Some(&previous));
        assert_eq!(joined, fresh);
        assert!(!Arc::ptr_eq(&joined.stable_ids, &previous.stable_ids));
        assert_eq!(previous.as_ref(), &original);
    }
}

#[test]
fn relocation_of_a_shared_summary_preserves_old_coordinates() {
    let previous = summary(":: a default\n> a@11111111111111111111\n  One.\n-> END\n");
    let original = previous.as_ref().clone();
    let mut relocated = previous.as_ref().clone();
    let ranges = relocated.region_ranges();
    relocated.relocate_region(&ranges, |span| {
        span.start =
            recite_core::SourcePosition::new(span.start.line() + 1, span.start.column()).unwrap();
        span.end = span
            .end
            .map(|end| recite_core::SourcePosition::new(end.line() + 1, end.column()).unwrap());
    });
    assert_eq!(previous.as_ref(), &original);
    assert!(!Arc::ptr_eq(&relocated.stable_ids, &previous.stable_ids));
    assert_eq!(
        relocated.stable_ids[0].span.start.line(),
        original.stable_ids[0].span.start.line() + 1
    );
}
