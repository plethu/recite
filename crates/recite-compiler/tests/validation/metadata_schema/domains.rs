#[test]
fn validates_flat_and_contextual_metadata_domains() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> intro@fcbba9f67b73ef5bcc1f portrait_domain=neutral subject=rhea emotion=calm tags=[flat, neutral]\n",
            "  Hello.\n",
            "> explicit@efe6cc3c3057d6f36d3a speaker=rhea portrait_domain=flat\n",
            "  Hello.\n",
            "> fallback@4d873c2b07e41eb549c3 portrait_domain=neutral\n",
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
fn contextual_value_names_are_resolved_by_context_and_fallback_policy() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: contextual default speaker=hazel\n",
            "> same_name@11111111111111111111 speaker=hazel portrait_domain=hazel\n",
            "  Same name is valid in the hazel context.\n",
            "> rejected@22222222222222222222 speaker=rhea portrait_domain=hazel\n",
            "  Same name is not valid in the rhea context.\n",
            ":: fallback\n",
            "> fallback@33333333333333333333 portrait_domain=neutral\n",
            "  Missing context uses the declared flat fallback.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE031"]);
    assert_spans(&report, [(4, 62)]);
}

#[test]
fn reports_invalid_metadata_domain_values_on_value_spans() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=rhea\n",
            "> intro@4d62d02c2f1e53710a5a portrait_domain=neutral tags=[flat, missing]\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE031", "RECITE_VALIDATE031"]);
    assert_spans(&report, [(2, 46), (2, 59)]);
}

#[test]
fn reports_missing_and_malformed_metadata_domain_context() {
    let schema = metadata_domain_schema();
    let missing = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro@11111111111111111111 emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&missing, &schema);
    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(2, 38)]);

    let malformed = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro@11111111111111111111 subject=\"rhea\" emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&malformed, &schema);
    assert_codes(&report, ["RECITE_VALIDATE029", "RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 38), (2, 38)]);
}

#[test]
fn block_metadata_does_not_use_default_speaker_as_field_speaker_context() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default speaker=hazel block_context=flat\n> intro@11111111111111111111\n  Hello.\n",
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
        ":: start default\n> intro@11111111111111111111 subject=rhea subject=hazel emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 43)]);
}

#[test]
fn reports_mixed_array_metadata_type_mismatch_before_domain_validation() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro@11111111111111111111 tags=[flat, \"neutral\"]\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE029"]);
    assert_spans(&report, [(2, 35)]);
}
use super::super::*;
use super::support::metadata_domain_schema;
