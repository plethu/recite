use recite_core::schema::{ProjectSchema, load_schema_manifest_str};

use super::*;

fn generated_manifest_schema() -> ProjectSchema {
    load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid generated manifest fixture")
}

#[test]
fn validates_choice_requires_and_reason_against_generated_manifest_schema() {
    let schema = generated_manifest_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint @b34jwqm\n",
            "  What's the news?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert!(report.is_ok(), "valid availability should pass: {report:?}");
}

#[test]
fn reports_unknown_condition_function_on_function_span() {
    let schema = generated_manifest_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? requires=(missing_condition(hazel)) @ybtdmgg\n",
            "  What's the news?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE034"]);
    assert_spans(&report, [(2, 13)]);
}

#[test]
fn reports_condition_arity_type_and_value_errors_on_argument_spans() {
    let schema = generated_manifest_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? requires=(trust_gte(hazel, rhea)) @ask_short\n",
            "  Short?\n",
            "  -> END\n",
            "? requires=(trust_gte(hazel, rhea, \"three\")) @ask_type\n",
            "  Type?\n",
            "  -> END\n",
            "? requires=(trust_gte(ghost, rhea, 3)) @ask_value\n",
            "  Value?\n",
            "  -> END\n",
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
            "RECITE_VALIDATE035",
            "RECITE_VALIDATE036",
            "RECITE_VALIDATE037",
        ],
    );
    assert_spans(&report, [(2, 13), (5, 36), (8, 23)]);
}

#[test]
fn reports_non_bool_conditions_for_if_and_requires_and_bool_scrutinee_for_match() {
    let schema = generated_manifest_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if thread_stage(hazel_intro)\n",
            "  > @gated\n",
            "    Gated.\n",
            "? requires=(thread_stage(hazel_intro)) @ask_stage\n",
            "  Stage?\n",
            "  -> END\n",
            ":match trust_gte(hazel, rhea, 3)\n",
            "  :case _\n",
            "    > @fallback\n",
            "      Fallback.\n",
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
            "RECITE_VALIDATE038",
            "RECITE_VALIDATE038",
            "RECITE_VALIDATE038",
        ],
    );
    assert_spans(&report, [(2, 5), (5, 13), (8, 8)]);
}

#[test]
fn reports_availability_reason_override_errors() {
    let schema = generated_manifest_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? requires=(trust_gte(hazel, rhea, 3)) reason=missing_reason @unknown\n",
            "  Unknown?\n",
            "  -> END\n",
            "? requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low @parameterized_definition\n",
            "  Definition?\n",
            "  -> END\n",
            "? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint(hazel) @parameterized_syntax\n",
            "  Syntax?\n",
            "  -> END\n",
            "? reason=innkeeper_trust_hint @no_requires\n",
            "  No requires?\n",
            "  -> END\n",
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
            "RECITE_VALIDATE039",
            "RECITE_VALIDATE040",
            "RECITE_VALIDATE040",
            "RECITE_VALIDATE041",
        ],
    );
    assert_spans(&report, [(2, 47), (5, 47), (8, 67), (11, 3)]);
}
