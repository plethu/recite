#[test]
fn accepts_schema_declared_metadata_on_supported_targets() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=room\n",
            "> intro@7c622c57e0fdfbf21758 speaker=hazel portrait=\"neutral\" caption=\"Hello.\" mood=calm priority=3 weight=1.5 flag=true route=north talker=rhea sfx=snap sfx=door_close\n",
            "  Hello.\n",
            "  ? ask@7d0cde9fe57d7447e899 sfx=snap\n",
            "    Ask.\n",
            "    -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "valid metadata should pass: {report:?}");
}
#[test]
fn reports_unknown_metadata_key_on_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro@11111111111111111111 speaker=hazel mystery=flat\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE026"]);
    assert_spans(&report, [(2, 44)]);
}

#[test]
fn reports_invalid_metadata_target_on_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default portrait=\"neutral\"\n> intro@11111111111111111111 speaker=hazel\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE027"]);
    assert_spans(&report, [(1, 18)]);
}

#[test]
fn reports_non_repeatable_duplicate_metadata_on_duplicate_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro@11111111111111111111 speaker=hazel portrait=\"neutral\" portrait=\"flat\"\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE028"]);
    assert_spans(&report, [(2, 63)]);
}

#[test]
fn reports_scalar_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro@a435f44e807666e33269 speaker=hazel priority=\"high\" weight=heavy flag=yes portrait=[flat] caption=plain\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

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
    assert_spans(&report, [(2, 53), (2, 67), (2, 78), (2, 91), (2, 106)]);
}

#[test]
fn reports_quoted_reference_and_symbol_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=\"room\"\n",
            "> intro@bbc9e1b9bd1560a8f62d speaker=hazel talker=\"rhea\" mood=\"calm\" sfx=\"snap\" route=\"north\"\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

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
    assert_spans(&report, [(1, 28), (2, 51), (2, 63), (2, 74), (2, 87)]);
}

#[test]
fn reports_invalid_speaker_enum_and_registry_metadata_values_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro@0ece4970482f210f1dd5 speaker=hazel talker=ghost mood=angry sfx=missing\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
        ],
    );
    assert_spans(&report, [(2, 51), (2, 62), (2, 72)]);
}

#[test]
fn skips_metadata_schema_validation_without_schema() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default portrait=neutral\n",
            "> intro@8bb522cb407f7b2f481c speaker=hazel mystery=flat portrait=[flat]\n",
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
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro@5a7ed0a8a6d5622db244 speaker=hazel route=north\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "symbol metadata should pass: {report:?}");
}
use super::super::*;
use super::support::metadata_schema;
