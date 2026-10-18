use super::super::AuthoringSummary;
use std::sync::Arc;

fn summary(source: &str) -> Arc<AuthoringSummary> {
    let lowered = recite_parser::parse("a.recite", source).lower_source_file();
    Arc::new(AuthoringSummary::from_source_file(&lowered.source_file))
}

#[test]
fn reference_changes_share_all_unchanged_record_families() {
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
    assert!(Arc::ptr_eq(&joined.blocks, &previous.blocks));
    assert!(Arc::ptr_eq(&joined.metadata, &previous.metadata));
    assert!(Arc::ptr_eq(
        &joined.condition_functions,
        &previous.condition_functions
    ));
    assert!(Arc::ptr_eq(
        &joined.effect_functions,
        &previous.effect_functions
    ));
    assert!(!Arc::ptr_eq(
        &joined.block_references,
        &previous.block_references
    ));
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
fn changing_one_family_preserves_the_others_and_relocation_separates_every_span_owner() {
    let source = ":: a default tag=old tags=[one,two]\n> a@11111111111111111111\n  One.\n:if condition()\n  ! immediate effect()\n-> missing\n";
    let previous = summary(source);
    assert!(!previous.metadata.is_empty());
    assert!(
        previous
            .metadata
            .iter()
            .any(|item| !item.value_element_spans.is_empty())
    );
    assert!(!previous.condition_functions.is_empty());
    assert!(!previous.effect_functions.is_empty());
    for (from, to, metadata, condition, effect) in [
        ("tag=old", "tag=new", true, false, false),
        ("condition()", "renamed()", false, true, false),
        ("effect()", "renamed()", false, false, true),
    ] {
        let fresh = summary(&source.replace(from, to));
        let joined =
            AuthoringSummary::join_regions(&[(&fresh, &fresh.region_ranges())], Some(&previous));
        assert_eq!(joined, fresh);
        assert_eq!(Arc::ptr_eq(&joined.metadata, &previous.metadata), !metadata);
        assert_eq!(
            Arc::ptr_eq(&joined.condition_functions, &previous.condition_functions),
            !condition
        );
        assert_eq!(
            Arc::ptr_eq(&joined.effect_functions, &previous.effect_functions),
            !effect
        );
        assert!(Arc::ptr_eq(&joined.blocks, &previous.blocks));
        assert!(Arc::ptr_eq(&joined.stable_ids, &previous.stable_ids));
        assert!(Arc::ptr_eq(
            &joined.block_references,
            &previous.block_references
        ));
    }
    let original = previous.as_ref().clone();
    let mut relocated = original.clone();
    let ranges = relocated.region_ranges();
    relocated.relocate_region(&ranges, |span| {
        span.start =
            recite_core::SourcePosition::new(span.start.line() + 1, span.start.column()).unwrap();
        span.end = span
            .end
            .map(|end| recite_core::SourcePosition::new(end.line() + 1, end.column()).unwrap());
    });
    assert_eq!(&relocated, summary(&format!("\n{source}")).as_ref());
    assert_eq!(previous.as_ref(), &original);
    assert!(!Arc::ptr_eq(&relocated.blocks, &previous.blocks));
    assert!(!Arc::ptr_eq(
        &relocated.block_references,
        &previous.block_references
    ));
    assert!(!Arc::ptr_eq(&relocated.metadata, &previous.metadata));
    assert!(!Arc::ptr_eq(
        &relocated.condition_functions,
        &previous.condition_functions
    ));
    assert!(!Arc::ptr_eq(
        &relocated.effect_functions,
        &previous.effect_functions
    ));
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
