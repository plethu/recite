#[test]
fn validates_flat_and_contextual_metadata_domains() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> portrait_domain=neutral subject=rhea emotion=calm tags=[flat, neutral] @mhd02nc\n",
            "  Hello.\n",
            "> speaker=rhea portrait_domain=flat @explicit\n",
            "  Hello.\n",
            "> portrait_domain=neutral @fallback\n",
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

    assert!(
        report.is_ok(),
        "valid metadata domains should pass: {report:?}"
    );
}

#[test]
fn contextual_value_names_are_resolved_by_context_and_fallback_policy() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: contextual default speaker=hazel\n",
            "> speaker=hazel portrait_domain=hazel @ffee6p8\n",
            "  Same name is valid in the hazel context.\n",
            "> speaker=rhea portrait_domain=hazel @rthh7ab\n",
            "  Same name is not valid in the rhea context.\n",
            ":: fallback\n",
            "> portrait_domain=neutral @bqyvpqq\n",
            "  Missing context uses the declared flat fallback.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE031"]);
    assert_spans(&report, [(4, 32)]);
}

#[test]
fn reports_invalid_metadata_domain_values_on_value_spans() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=rhea\n",
            "> portrait_domain=neutral tags=[flat, missing] @jtxwznr\n",
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

    assert_codes(&report, ["RECITE_VALIDATE031", "RECITE_VALIDATE031"]);
    assert_spans(&report, [(2, 19), (2, 32)]);
}

#[test]
fn reports_missing_and_malformed_metadata_domain_context() {
    let schema = metadata_domain_schema();
    let missing = [lower(
        "dialogue/start.recite",
        ":: start default\n> emotion=calm @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        missing
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );
    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(2, 11)]);

    let malformed = [lower(
        "dialogue/start.recite",
        ":: start default\n> subject=\"rhea\" emotion=calm @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        malformed
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );
    assert_codes(&report, ["RECITE_VALIDATE029", "RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 11), (2, 11)]);
}

#[test]
fn block_metadata_does_not_use_default_speaker_as_field_speaker_context() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default speaker=hazel block_context=flat\n> @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(1, 46)]);
}

#[test]
fn repeated_metadata_selector_reports_selector_span() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default\n> subject=rhea subject=hazel emotion=calm @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 16)]);
}

#[test]
fn reports_mixed_array_metadata_type_mismatch_before_domain_validation() {
    let schema = metadata_domain_schema();
    let files = [lower(
        "dialogue/start.recite",
        ":: start default\n> tags=[flat, \"neutral\"] @ffee6p8\n  Hello.\n",
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE029"]);
    assert_spans(&report, [(2, 8)]);
}
use super::super::*;
use super::support::metadata_domain_schema;
