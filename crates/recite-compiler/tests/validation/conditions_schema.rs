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
            "? ask_news@3b4e90e832dca4523ff1 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
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
            "? ask_news@69708df132b882043039 requires=(missing_condition(hazel))\n",
            "  What's the news?\n",
            "  -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE034"]);
    assert_spans(&report, [(2, 43)]);
}

#[test]
fn reports_condition_arity_type_and_value_errors_on_argument_spans() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? ask_short@22eba876e73e3a583ec7 requires=(trust_gte(hazel, rhea))\n",
            "  Short?\n",
            "  -> END\n",
            "? ask_type@a5a3c39f0d18fab3472c requires=(trust_gte(hazel, rhea, \"three\"))\n",
            "  Type?\n",
            "  -> END\n",
            "? ask_value@9968a9c829c382415c71 requires=(trust_gte(ghost, rhea, 3))\n",
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
    assert_spans(&report, [(2, 44), (5, 66), (8, 54)]);
}

#[test]
fn reports_non_bool_conditions_for_if_and_requires_and_bool_scrutinee_for_match() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if thread_stage(hazel_intro)\n",
            "  > gated@9ccb225ac6eedef131f1\n",
            "    Gated.\n",
            "? ask_stage@23cc94798fac47d3d1a3 requires=(thread_stage(hazel_intro))\n",
            "  Stage?\n",
            "  -> END\n",
            ":match trust_gte(hazel, rhea, 3)\n",
            "  :case _\n",
            "    > fallback@3d6c861f03fb7bd905ac\n",
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
    assert_spans(&report, [(2, 5), (5, 44), (8, 8)]);
}

#[test]
fn reports_availability_reason_override_errors() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "? unknown@6eda530ac11847c6b361 requires=(trust_gte(hazel, rhea, 3)) reason=missing_reason\n",
            "  Unknown?\n",
            "  -> END\n",
            "? parameterized_definition@8a3fc7995329e0e3e5af requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low\n",
            "  Definition?\n",
            "  -> END\n",
            "? parameterized_syntax@778d2573084e8e8ff33d requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint(hazel)\n",
            "  Syntax?\n",
            "  -> END\n",
            "? no_requires@c6ce98af99068a6fda55 reason=innkeeper_trust_hint\n",
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
    assert_spans(&report, [(2, 76), (5, 93), (8, 109), (11, 36)]);
}
