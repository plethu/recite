use super::*;

#[test]
fn validates_source_paths_are_unique_before_asset_output() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(":: start default\n", "> first\n", "  First.\n",),
        ),
        lower(
            "dialogue/start.recite",
            concat!(":: other\n", "> second\n", "  Second.\n",),
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
            concat!(":: shared default\n", "> first\n", "  First.\n",),
        ),
        lower(
            "dialogue/b.recite",
            concat!(":: shared\n", "> second\n", "  Second.\n",),
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
        concat!(":: start default\n", "? choose\n", "  Choose.\n",),
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
            "> prompt\n",
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
            "? choose\n",
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
            "? choose echo=line(missing_echo_line)\n",
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
            "? choose echo=line(echo_line)\n",
            "  Choose.\n",
            "  -> END\n",
            "> echo_line\n",
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
            "> line line_value=inf array_value=[1, -inf]\n",
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
    assert_spans(&report, [(1, 30), (2, 19), (2, 35)]);
}
