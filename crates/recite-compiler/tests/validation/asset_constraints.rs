use super::*;

#[test]
fn validates_source_paths_are_unique_before_asset_output() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "> first@11111111111111111111\n",
                "  First.\n",
            ),
        ),
        lower(
            "dialogue/start.recite",
            concat!(
                ":: other\n",
                "> second@22222222222222222222\n",
                "  Second.\n",
            ),
        ),
    ];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE010"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 1);
    assert_eq!(report.diagnostics[0].related[0].span.start.line(), 1);
}

#[test]
fn validates_compiled_block_ids_are_globally_unambiguous_for_v0_lookup() {
    let files = vec![
        lower(
            "dialogue/a.recite",
            concat!(
                ":: shared default\n",
                "> first@11111111111111111111\n",
                "  First.\n",
            ),
        ),
        lower(
            "dialogue/b.recite",
            concat!(
                ":: shared\n",
                "> second@22222222222222222222\n",
                "  Second.\n",
            ),
        ),
    ];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE011"]);
    assert_eq!(report.diagnostics[0].span.file, "dialogue/b.recite");
    assert_eq!(
        report.diagnostics[0].related[0].span.file,
        "dialogue/a.recite"
    );
}

#[test]
fn validates_choices_have_compile_targets() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? choose@11111111111111111111\n",
            "  Choose.\n",
        ),
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
            "> prompt@a026173461e6faac45b9\n",
            "  Prompt.\n",
            "  ! immediate play_sfx(chime)\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE013"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(report.diagnostics[0].related[0].span.start.line(), 2);
}

#[test]
fn validates_choice_bodies_do_not_leave_runtime_unrepresentable_children() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? choose@aaee1e82b688f8d62d56\n",
            "  Choose.\n",
            "  -> END\n",
            "  ! immediate play_sfx(chime)\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE014"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
    assert_eq!(report.diagnostics[0].related[0].span.start.line(), 2);
}

#[test]
fn validates_choice_echo_lines_exist_before_asset_output() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? choose@5e85f5b4ad15bb59a55d echo=line(947b5cc648174c8cabd1)\n",
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
            "? choose@db4d20bec509c2ac56b6 echo=line(a45a5dab466f6c482701)\n",
            "  Choose.\n",
            "  -> END\n",
            "> echo_line@a45a5dab466f6c482701\n",
            "  Echo.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(report.is_ok(), "later line IDs should be valid: {report:?}");
}

#[test]
fn validates_metadata_floats_are_finite_for_v0_inspection_output() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_value=NaN\n",
            "> line@8b9cb9816ec5798c5178 line_value=inf array_value=[1, -inf]\n",
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
    assert_spans(&report, [(1, 30), (2, 40), (2, 56)]);
}
