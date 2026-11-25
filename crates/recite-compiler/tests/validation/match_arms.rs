use super::*;

fn match_source(arms: &str) -> String {
    format!(":: start default\n:match mood(player)\n{arms}-> END\n")
}

#[test]
fn rejects_a_repeated_variant_arm_and_points_at_the_first() {
    let source = match_source(concat!(
        "  :case tired\n",
        "    -> END\n",
        "  :case calm\n",
        "    -> END\n",
        "  :case tired\n",
        "    -> END\n",
    ));
    let report = validate_source_files(&[lower("dialogue/start.recite", &source)]);
    assert_codes(&report, ["RECITE_VALIDATE052"]);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.span.start.line(), 7);
    assert_eq!(diagnostic.related_presentations[0].span.start.line(), 3);
}

#[test]
fn rejects_any_arm_after_the_wildcard_including_a_second_wildcard() {
    let source = match_source(concat!(
        "  :case _\n",
        "    -> END\n",
        "  :case tired\n",
        "    -> END\n",
        "  :case _\n",
        "    -> END\n",
    ));
    let report = validate_source_files(&[lower("dialogue/start.recite", &source)]);
    assert_codes(&report, ["RECITE_VALIDATE053", "RECITE_VALIDATE053"]);
    for diagnostic in &report.diagnostics {
        assert_eq!(diagnostic.related_presentations[0].span.start.line(), 3);
    }
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
    assert_eq!(report.diagnostics[1].span.start.line(), 7);
}

#[test]
fn validates_nested_matches_independently() {
    let source = match_source(concat!(
        "  :case tired\n",
        "    :match mood(guard)\n",
        "      :case tired\n",
        "        -> END\n",
        "      :case tired\n",
        "        -> END\n",
        "  :case calm\n",
        "    -> END\n",
        "  :case _\n",
        "    -> END\n",
    ));
    let report = validate_source_files(&[lower("dialogue/start.recite", &source)]);
    // The inner repeat is reported; the outer `tired` arm does not clash
    // with the inner one.
    assert_codes(&report, ["RECITE_VALIDATE052"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 7);
    assert_eq!(
        report.diagnostics[0].related_presentations[0]
            .span
            .start
            .line(),
        5
    );
}
