use recite_core::{ProjectSchema, load_schema_manifest_str};

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
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? ask_news requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "  What's the news?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "valid availability should pass: {report:?}");
}

#[test]
fn reports_unknown_condition_function_on_function_span() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? ask_news requires=(missing_condition(hazel))\n",
            "  What's the news?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE034"]);
    assert_spans(&report, [(2, 22)]);
}

#[test]
fn reports_condition_arity_type_and_value_errors_on_argument_spans() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? ask_short requires=(trust_gte(hazel, rhea))\n",
            "  Short?\n",
            "  -> END\n",
            "? ask_type requires=(trust_gte(hazel, rhea, \"three\"))\n",
            "  Type?\n",
            "  -> END\n",
            "? ask_value requires=(trust_gte(ghost, rhea, 3))\n",
            "  Value?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE035",
            "RECITE_VALIDATE036",
            "RECITE_VALIDATE037",
        ],
    );
    assert_spans(&report, [(2, 23), (5, 45), (8, 33)]);
}

#[test]
fn reports_non_bool_conditions_for_if_and_requires_and_bool_scrutinee_for_match() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if thread_stage(hazel_intro)\n",
            "  > gated\n",
            "    Gated.\n",
            "? ask_stage requires=(thread_stage(hazel_intro))\n",
            "  Stage?\n",
            "  -> END\n",
            ":match trust_gte(hazel, rhea, 3)\n",
            "  :case _\n",
            "    > fallback\n",
            "      Fallback.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE038",
            "RECITE_VALIDATE038",
            "RECITE_VALIDATE038",
        ],
    );
    assert_spans(&report, [(2, 5), (5, 23), (8, 8)]);
}

#[test]
fn reports_availability_reason_override_errors() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? unknown requires=(trust_gte(hazel, rhea, 3)) reason=missing_reason\n",
            "  Unknown?\n",
            "  -> END\n",
            "? parameterized_definition requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low\n",
            "  Definition?\n",
            "  -> END\n",
            "? parameterized_syntax requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint(hazel)\n",
            "  Syntax?\n",
            "  -> END\n",
            "? no_requires reason=innkeeper_trust_hint\n",
            "  No requires?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE039",
            "RECITE_VALIDATE040",
            "RECITE_VALIDATE040",
            "RECITE_VALIDATE041",
        ],
    );
    assert_spans(&report, [(2, 55), (5, 72), (8, 88), (11, 15)]);
}
