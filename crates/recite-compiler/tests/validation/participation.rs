use recite_compiler::{
    Participation, ValidationCompleteness, ValidationParticipation, ValidationSourceFile,
    validate_source_files, validate_source_files_with_participation,
    validate_source_files_with_participation_with_schema, validate_source_files_with_schema,
};
use recite_core::{ProjectSchema, load_schema_manifest_str};
use recite_parser::parse;

fn participated<'a>(source_file: &'a recite_core::SourceFile) -> ValidationSourceFile<'a> {
    ValidationSourceFile::all_complete(source_file)
}

fn lower_clean(path: &str, source: &str) -> recite_core::SourceFile {
    let lowered = parse(path, source).lower_source_file();
    assert!(
        lowered.diagnostics.is_empty(),
        "test fixture must parse/lower cleanly: {:?}",
        lowered.diagnostics
    );
    lowered.source_file
}

fn generated_manifest_schema() -> ProjectSchema {
    load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid generated manifest fixture")
}

fn codes(report: &recite_compiler::ValidationReport) -> Vec<&str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn all_complete_participation_preserves_schema_and_schema_free_reports() {
    let schema = generated_manifest_schema();
    let source = lower_clean(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first@11111111111111111111 portrait=1\n",
            "  [unknown]Hello {name}.\n",
            "! immediate missing_effect()\n",
        ),
    );
    let files = [source];
    let inputs = [participated(&files[0])];

    assert_eq!(
        validate_source_files(&files),
        validate_source_files_with_participation(&inputs)
    );
    assert_eq!(
        validate_source_files_with_schema(&files, &schema),
        validate_source_files_with_participation_with_schema(&inputs, &schema)
    );
}

#[test]
fn all_complete_participation_preserves_ordering_for_reversed_inputs() {
    let schema = generated_manifest_schema();
    let first = lower_clean(
        "dialogue/zeta.recite",
        concat!(
            ":: zeta default\n",
            "> first@11111111111111111111 unknown=1\n",
            "  [unknown]Hello.\n",
        ),
    );
    let second = lower_clean(
        "dialogue/alpha.recite",
        concat!(
            ":: alpha\n",
            "> second@22222222222222222222\n",
            "  Hello.\n",
        ),
    );
    let files = [first, second];
    let forward = [participated(&files[0]), participated(&files[1])];
    let reversed = [participated(&files[1]), participated(&files[0])];

    assert_eq!(
        validate_source_files(&files),
        validate_source_files_with_participation(&forward)
    );
    assert_eq!(
        validate_source_files_with_schema(&files, &schema),
        validate_source_files_with_participation_with_schema(&reversed, &schema)
    );
}

#[test]
fn incomplete_target_definitions_make_reference_lookup_indeterminate() {
    let source = lower_clean(
        "dialogue/start.recite",
        ":: start default\n-> dialogue/target.recite::missing\n",
    );
    let target = lower_clean("dialogue/target.recite", ":: target\n");
    let mut target_participation = ValidationParticipation::all_complete();
    target_participation.block_definitions = Participation::Incomplete;
    let files = [source, target];
    let inputs = [
        participated(&files[0]),
        ValidationSourceFile::new(&files[1], target_participation),
    ];

    assert!(
        validate_source_files_with_participation(&inputs)
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.as_str() != "RECITE_VALIDATE007")
    );

    let complete = [
        ValidationSourceFile::all_complete(&files[0]),
        ValidationSourceFile::all_complete(&files[1]),
    ];
    let report = validate_source_files_with_participation(&complete);
    assert_eq!(codes(&report), ["RECITE_VALIDATE007"]);
}

#[test]
fn incomplete_classes_do_not_suppress_unrelated_clean_file_diagnostics() {
    let clean = lower_clean(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> line@11111111111111111111 unknown=1\n",
            "  [unknown]Hello.\n",
            ":if missing_condition()\n",
            "  > gated@22222222222222222222\n",
            "    Gated.\n",
            "! immediate missing_effect()\n",
        ),
    );
    let unrelated = lower_clean(
        "dialogue/other.recite",
        concat!(
            ":: other\n",
            "> bad@33333333333333333333 unknown=1\n",
            "  [ghost]Other.\n",
            ":if missing_condition()\n",
            "  > nested@44444444444444444444\n",
            "    Nested.\n",
            "! immediate missing_effect()\n",
        ),
    );
    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.metadata = Participation::Incomplete;
    incomplete.condition_functions = Participation::Incomplete;
    incomplete.effect_functions = Participation::Incomplete;
    incomplete.inline_markup = Participation::Incomplete;
    let files = [clean, unrelated];
    let inputs = [
        participated(&files[0]),
        ValidationSourceFile::new(&files[1], incomplete),
    ];

    let report =
        validate_source_files_with_participation_with_schema(&inputs, &ProjectSchema::empty_v1());
    assert_eq!(
        codes(&report),
        [
            "RECITE_VALIDATE026",
            "RECITE_VALIDATE022",
            "RECITE_VALIDATE034",
            "RECITE_VALIDATE017",
        ]
    );
}

#[test]
fn incomplete_ast_structure_suppresses_interpolation_diagnostics() {
    let source = lower_clean(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> line@11111111111111111111\n",
            "  Hello {missing}.\n",
        ),
    );
    let complete = [participated(&source)];
    assert_eq!(
        codes(&validate_source_files_with_participation(&complete)),
        ["RECITE_VALIDATE045"]
    );

    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.ast_structure = Participation::Incomplete;
    let input = [ValidationSourceFile::new(&source, incomplete)];
    assert!(
        validate_source_files_with_participation(&input)
            .diagnostics
            .is_empty()
    );
}

#[test]
fn incomplete_stable_ids_do_not_contribute_duplicate_or_echo_evidence() {
    let first = lower_clean(
        "dialogue/first.recite",
        ":: first default\n> same@11111111111111111111\n  First.\n",
    );
    let second = lower_clean(
        "dialogue/second.recite",
        ":: second\n> same@11111111111111111111\n  Second.\n",
    );
    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.stable_ids = Participation::Incomplete;
    let files = [first, second];
    let inputs = [
        participated(&files[0]),
        ValidationSourceFile::new(&files[1], incomplete),
    ];

    let report = validate_source_files_with_participation(&inputs);
    assert!(
        codes(&report).is_empty(),
        "unexpected diagnostics: {report:?}"
    );

    let complete = [participated(&files[0]), participated(&files[1])];
    let report = validate_source_files_with_participation(&complete);
    assert_eq!(codes(&report), ["RECITE_ID003"]);
}

#[test]
fn incomplete_block_definitions_do_not_contribute_duplicate_evidence() {
    let first = lower_clean(
        "dialogue/first.recite",
        ":: shared default\n> first@11111111111111111111\n  First.\n",
    );
    let second = lower_clean(
        "dialogue/second.recite",
        ":: shared\n> second@22222222222222222222\n  Second.\n",
    );
    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.block_definitions = Participation::Incomplete;
    let files = [first, second];
    let inputs = [
        participated(&files[0]),
        ValidationSourceFile::new(&files[1], incomplete),
    ];

    let report = validate_source_files_with_participation(&inputs);
    assert!(
        codes(&report).is_empty(),
        "unexpected diagnostics: {report:?}"
    );

    let complete = [participated(&files[0]), participated(&files[1])];
    let report = validate_source_files_with_participation(&complete);
    assert_eq!(codes(&report), ["RECITE_VALIDATE011"]);
    assert_eq!(
        report.diagnostics[0].related_presentations[0].span.file,
        "dialogue/first.recite"
    );
}

#[test]
fn incomplete_definitions_suppress_missing_default_but_complete_defaults_conflict() {
    let source = lower_clean("dialogue/start.recite", ":: start\n");
    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.block_definitions = Participation::Incomplete;
    let files = [source];
    let inputs = [ValidationSourceFile::new(&files[0], incomplete)];
    assert!(validate_source_files_with_participation(&inputs).is_ok());

    let complete = [participated(&files[0])];
    let report = validate_source_files_with_participation(&complete);
    assert_eq!(codes(&report), ["RECITE_VALIDATE005"]);
}

#[test]
fn unknown_choice_echo_is_indeterminate_until_all_stable_ids_are_complete() {
    let source = lower_clean(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> line@11111111111111111111\n",
            "  Line.\n",
            "? choose@22222222222222222222 echo=line(missing)\n",
            "  Choose.\n",
            "  -> END\n",
        ),
    );
    let other = lower_clean("dialogue/other.recite", ":: other\n");
    let mut incomplete = ValidationParticipation::all_complete();
    incomplete.stable_ids = ValidationCompleteness::Incomplete;
    let files = [source, other];
    let inputs = [
        participated(&files[0]),
        ValidationSourceFile::new(&files[1], incomplete),
    ];
    let report = validate_source_files_with_participation(&inputs);
    assert!(
        codes(&report).is_empty(),
        "unexpected diagnostics: {report:?}"
    );

    let complete = [participated(&files[0]), participated(&files[1])];
    let report = validate_source_files_with_participation(&complete);
    assert_eq!(codes(&report), ["RECITE_VALIDATE015"]);
}
