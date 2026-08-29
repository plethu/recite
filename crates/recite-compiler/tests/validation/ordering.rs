use super::*;

#[test]
fn empty_project_has_no_span_to_report() {
    assert!(validate_source_files(&[]).is_ok());
}

#[test]
fn diagnostics_are_sorted_by_canonical_source_order() {
    let files = vec![lower(
        "dialogue/order.recite",
        concat!(
            ":: start\n",
            "? choose_path@3cba055063924b49c4f1\n",
            "  Choose.\n",
            "  >\n",
            "    Nested missing line ID.\n",
            "  -> missing_target\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE005",
            "RECITE_ID001",
            "RECITE_VALIDATE014",
            "RECITE_VALIDATE007",
        ],
    );
    assert_spans(&report, [(1, 1), (4, 3), (4, 3), (6, 3)]);
}

#[test]
fn validation_is_independent_of_caller_file_order() {
    let first = lower(
        "dialogue/a.recite",
        concat!(
            ":: first default\n",
            "> shared@31c87ff9bdb89723be77\n",
            "  First.\n",
        ),
    );
    let second = lower(
        "dialogue/b.recite",
        concat!(
            ":: second default\n",
            "? shared@31c87ff9bdb89723be77\n",
            "  Second.\n",
            "  -> END\n",
        ),
    );

    let forward = validate_source_files(&[first.clone(), second.clone()]);
    let reverse = validate_source_files(&[second, first]);

    assert_eq!(forward, reverse);
    assert_codes(&forward, ["RECITE_VALIDATE006", "RECITE_ID004"]);
    assert_eq!(
        forward.diagnostics[0].related_presentations[0].span.file,
        "dialogue/a.recite"
    );
    assert_eq!(
        forward.diagnostics[1].related_presentations[0].span.file,
        "dialogue/a.recite"
    );
}

#[test]
fn line_and_choice_ids_share_one_localisable_namespace() {
    let files = vec![lower(
        "dialogue/shared.recite",
        concat!(
            ":: start default\n",
            "> shared@c1cb25f1a9db29fd0a62\n",
            "  Line.\n",
            "? shared@c1cb25f1a9db29fd0a62\n",
            "  Choice.\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_ID004"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(
        report.diagnostics[0].related_presentations[0]
            .span
            .start
            .line(),
        2
    );
}
