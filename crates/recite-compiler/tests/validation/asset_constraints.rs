use super::*;

#[test]
fn validates_source_paths_are_unique_before_asset_output() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(":: start default\n", "> @ffee6p8\n", "  First.\n",),
        ),
        lower(
            "dialogue/start.recite",
            concat!(":: other\n", "> @rthh7ab\n", "  Second.\n",),
        ),
    ];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE010"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 1);
    assert_eq!(
        report.diagnostics[0].related_presentations[0]
            .span
            .start
            .line(),
        1
    );
}

#[test]
fn validates_compiled_block_ids_are_globally_unambiguous_for_v0_lookup() {
    let files = vec![
        lower(
            "dialogue/a.recite",
            concat!(":: shared default\n", "> @ffee6p8\n", "  First.\n",),
        ),
        lower(
            "dialogue/b.recite",
            concat!(":: shared\n", "> @rthh7ab\n", "  Second.\n",),
        ),
    ];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE011"]);
    assert_eq!(report.diagnostics[0].span.file, "dialogue/b.recite");
    assert_eq!(
        report.diagnostics[0].related_presentations[0].span.file,
        "dialogue/a.recite"
    );
}

#[test]
fn validates_choices_have_compile_targets() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "? @ffee6p8\n", "  Choose.\n",),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE012"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 2);
}

#[test]
fn validates_prompt_line_children_are_choices_only_for_v0_assets() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @prompt\n",
            "  Prompt.\n",
            "  ! immediate play_sfx(chime)\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE013"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(
        report.diagnostics[0].related_presentations[0]
            .span
            .start
            .line(),
        2
    );
}

#[test]
fn validates_choice_bodies_do_not_leave_runtime_unrepresentable_children() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? @j17wfff\n",
            "  Choose.\n",
            "  -> END\n",
            "  ! immediate play_sfx(chime)\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE014"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
    assert_eq!(
        report.diagnostics[0].related_presentations[0]
            .span
            .start
            .line(),
        2
    );
}

#[test]
fn validates_choice_echo_lines_exist_before_asset_output() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? echo=line(947b5cc648174c8cabd1) @jxyshc4\n",
            "  Choose.\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE015"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 2);
}

#[test]
fn validates_choice_echo_can_reference_later_lines() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? echo=line(echo_line) @hmrpzd2\n",
            "  Choose.\n",
            "  -> END\n",
            "> @echo_line\n",
            "  Echo.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(report.is_ok(), "later line IDs should be valid: {report:?}");
}

#[test]
fn choice_echo_finds_later_nested_ids_even_when_choice_shape_is_invalid() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? echo=line(echo_line) @hmrpzd2\n",
            "  Choose.\n",
            "  -> END\n",
            "? @ffee6p8\n",
            "  Other.\n",
            "  > @echo_line\n",
            "    Echo.\n",
            "  -> END\n",
        ),
    )];
    let report = validate_source_files(&files);
    assert_codes(&report, ["RECITE_VALIDATE014"]);
}

#[test]
fn validates_metadata_floats_are_finite_for_v0_inspection_output() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_value=NaN\n",
            "> line_value=inf array_value=[1, -inf] @b5nrmqr\n",
            "  Text.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE016",
            "RECITE_VALIDATE016",
            "RECITE_VALIDATE016",
        ],
    );
    assert_spans(&report, [(1, 30), (2, 14), (2, 30)]);
}
