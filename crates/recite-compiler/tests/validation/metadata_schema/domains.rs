#[test]
fn validates_flat_and_contextual_metadata_domains() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> intro portrait_domain=neutral subject=rhea emotion=calm tags=[flat, neutral]\n",
            "  Hello.\n",
            "> explicit speaker=rhea portrait_domain=flat\n",
            "  Hello.\n",
            "> fallback portrait_domain=neutral\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(
        report.is_ok(),
        "valid metadata domains should pass: {report:?}"
    );
}

#[test]
fn reports_invalid_metadata_domain_values_on_value_spans() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=rhea\n",
            "> intro portrait_domain=neutral tags=[flat, missing]\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE031", "RECITE_VALIDATE031"]);
    assert_spans(&report, [(2, 25), (2, 38)]);
}

#[test]
fn reports_missing_and_malformed_metadata_domain_context() {
    let schema = metadata_domain_schema();
    let missing = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&missing, &schema);
    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(2, 17)]);

    let malformed = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro subject=\"rhea\" emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&malformed, &schema);
    assert_codes(&report, ["RECITE_VALIDATE029", "RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 17), (2, 17)]);
}

#[test]
fn block_metadata_does_not_use_default_speaker_as_field_speaker_context() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default speaker=hazel block_context=flat\n> intro\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(1, 46)]);
}

#[test]
fn repeated_metadata_selector_reports_selector_span() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro subject=rhea subject=hazel emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 22)]);
}

#[test]
fn reports_mixed_array_metadata_type_mismatch_before_domain_validation() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro tags=[flat, \"neutral\"]\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE029"]);
    assert_spans(&report, [(2, 14)]);
}
use super::super::*;
use super::support::metadata_domain_schema;
