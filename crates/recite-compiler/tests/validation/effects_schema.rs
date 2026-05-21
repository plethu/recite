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
fn validates_effects_against_generated_manifest_schema() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate play_sfx(snap)\n",
            "! deferred advance_thread(hazel_intro, fresh)\n",
            "! blocking advance_thread(hazel_intro, completed)\n",
            "! immediate scalar_effect(\"label\", 3, 1.5, true)\n",
            "! immediate speaker_effect(hazel)\n",
            "> line\n",
            "  Done.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "valid effects should pass: {report:?}");
}

#[test]
fn reports_unknown_effect_function_on_function_span() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "! immediate missing_effect(snap)\n"),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE017"]);
    assert_spans(&report, [(2, 13)]);
}

#[test]
fn reports_wrong_arity_on_the_smallest_useful_span() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred advance_thread(hazel_intro)\n",
            "! deferred advance_thread(hazel_intro, fresh, extra)\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE018", "RECITE_VALIDATE018"]);
    assert_spans(&report, [(2, 12), (3, 47)]);
    assert_eq!(
        report.diagnostics[0].span.end.map(|end| end.column()),
        Some(38)
    );
}

#[test]
fn reports_unsupported_effect_mode_on_mode_span() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "! blocking play_sfx(snap)\n"),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE020"]);
    assert_spans(&report, [(2, 3)]);
}

#[test]
fn reports_wrong_scalar_argument_types_on_argument_spans() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate scalar_effect(1, \"two\", true, flag)\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE019",
            "RECITE_VALIDATE019",
            "RECITE_VALIDATE019",
            "RECITE_VALIDATE019",
        ],
    );
    assert_spans(&report, [(2, 27), (2, 30), (2, 37), (2, 43)]);
}

#[test]
fn reports_invalid_speaker_registry_and_enum_values_on_argument_spans() {
    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate speaker_effect(ghost)\n",
            "! deferred advance_thread(missing_thread, unknown_stage)\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE021",
            "RECITE_VALIDATE021",
            "RECITE_VALIDATE021",
        ],
    );
    assert_spans(&report, [(2, 28), (3, 27), (3, 43)]);
}

#[test]
fn keeps_schema_manifest_diagnostics_distinct_from_dialogue_validation() {
    let malformed = load_schema_manifest_str("schemas/missing-or-bad.json", "{ not json");
    assert_eq!(
        malformed
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["RECITE_SCHEMA001"]
    );

    let schema = generated_manifest_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "! immediate missing_effect(snap)\n"),
    )];
    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE017"]);
}
