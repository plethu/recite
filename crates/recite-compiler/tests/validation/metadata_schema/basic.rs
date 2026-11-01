#[test]
fn accepts_schema_declared_metadata_on_supported_targets() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=room\n",
            "> speaker=hazel portrait=\"neutral\" caption=\"Hello.\" mood=calm priority=3 weight=1.5 flag=true route=north talker=rhea sfx=snap sfx=door_close @kd51sgw\n",
            "  Hello.\n",
            "  ? sfx=snap @ask\n",
            "    Ask.\n",
            "    -> END\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert!(report.is_ok(), "valid metadata should pass: {report:?}");
}
#[test]
fn reports_unknown_metadata_key_on_key_span() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default\n> speaker=hazel mystery=flat @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE026"]);
    assert_spans(&report, [(2, 17)]);
}

#[test]
fn reports_invalid_metadata_target_on_key_span() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default portrait=\"neutral\"\n> speaker=hazel @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE027"]);
    assert_spans(&report, [(1, 18)]);
}

#[test]
fn reports_non_repeatable_duplicate_metadata_on_duplicate_key_span() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default\n> speaker=hazel portrait=\"neutral\" portrait=\"flat\" @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE028"]);
    assert_spans(&report, [(2, 36)]);
}

#[test]
fn reports_scalar_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> speaker=hazel priority=\"high\" weight=heavy flag=yes portrait=[flat] caption=plain @x2htna1\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
        ],
    );
    assert_spans(&report, [(2, 26), (2, 40), (2, 51), (2, 64), (2, 79)]);
}

#[test]
fn reports_quoted_reference_and_symbol_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=\"room\"\n",
            "> speaker=hazel talker=\"rhea\" mood=\"calm\" sfx=\"snap\" route=\"north\" @z45efcj\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
        ],
    );
    assert_spans(&report, [(1, 28), (2, 24), (2, 36), (2, 47), (2, 60)]);
}

#[test]
fn reports_invalid_speaker_enum_and_registry_metadata_values_on_value_spans() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> speaker=hazel talker=ghost mood=angry sfx=missing @jqnt5gg\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
        ],
    );
    assert_spans(&report, [(2, 24), (2, 35), (2, 45)]);
}

#[test]
fn skips_metadata_schema_validation_without_schema() {
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default portrait=neutral\n",
            "> speaker=hazel mystery=flat portrait=[flat] @f0607h8\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(
        report.is_ok(),
        "schema-less metadata should pass: {report:?}"
    );
}

#[test]
fn accepts_metadata_only_symbol_type_on_line_metadata() {
    let schema = metadata_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> speaker=hazel route=north @wzexmp0\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert!(report.is_ok(), "symbol metadata should pass: {report:?}");
}
use super::super::*;
use super::support::metadata_schema;
