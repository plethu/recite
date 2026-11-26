use recite_core::schema::{ProjectSchema, load_schema_manifest_str};

use super::*;

fn mood_schema() -> ProjectSchema {
    load_schema_manifest_str(
        "schema/mood.schema.json",
        r#"{
  "schema_version": 1,
  "types": {"mood_kind": {"kind": "enum", "values": ["calm", "tired", "wary"]}},
  "conditions": {
    "mood": {"params": [], "returns": "enum:mood_kind"},
    "ready": {"params": []}
  }
}"#,
    )
    .schema
    .expect("valid schema")
}

fn validate_with(schema: Option<&ProjectSchema>, arms: &str) -> ValidationReport {
    let source = format!(":: start default\n:match mood()\n{arms}-> END\n");
    let files = [lower("dialogue/start.recite", &source)];
    validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        schema,
        recite_compiler::validation::ProjectCompleteness::Complete,
    )
}

#[test]
fn reports_an_undeclared_variant_with_a_suggestion_when_one_is_close() {
    let schema = mood_schema();
    let report = validate_with(
        Some(&schema),
        concat!(
            "  :case tird\n",
            "    -> END\n",
            "  :case sleepy\n",
            "    -> END\n",
            "  :case _\n",
            "    -> END\n",
        ),
    );
    assert_codes(&report, ["RECITE_VALIDATE054", "RECITE_VALIDATE054"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 3);
    let help = |index: usize| {
        report.diagnostics[index]
            .help_presentation
            .as_ref()
            .expect("help")
            .id()
            .as_str()
            .to_owned()
    };
    assert_eq!(help(0), "diagnostic-validate-054-help-suggestion");
    assert_eq!(help(1), "diagnostic-validate-054-help");
}

#[test]
fn reports_uncovered_variants_unless_a_wildcard_or_every_variant_covers_them() {
    let schema = mood_schema();
    let partial = validate_with(Some(&schema), "  :case tired\n    -> END\n");
    assert_codes(&partial, ["RECITE_VALIDATE055"]);
    assert_eq!(partial.diagnostics[0].span.start.line(), 2);
    assert!(
        partial.diagnostics[0].message.contains("`calm`, `wary`"),
        "{}",
        partial.diagnostics[0].message
    );

    // An undeclared arm covers nothing.
    let misspelt = validate_with(
        Some(&schema),
        "  :case calm\n    -> END\n  :case tird\n    -> END\n  :case wary\n    -> END\n",
    );
    assert_codes(&misspelt, ["RECITE_VALIDATE055", "RECITE_VALIDATE054"]);

    for arms in [
        "  :case tired\n    -> END\n  :case _\n    -> END\n",
        "  :case calm\n    -> END\n  :case tired\n    -> END\n  :case wary\n    -> END\n",
    ] {
        let report = validate_with(Some(&schema), arms);
        assert!(report.is_ok(), "{arms}: {report:?}");
    }
}

#[test]
fn nested_matches_are_each_checked() {
    let schema = mood_schema();
    let report = validate_with(
        Some(&schema),
        concat!(
            "  :case tired\n",
            "    :match mood()\n",
            "      :case wary\n",
            "        -> END\n",
            "  :case _\n",
            "    -> END\n",
        ),
    );
    assert_codes(&report, ["RECITE_VALIDATE055"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
}

#[test]
fn needs_a_schema_and_a_known_enum_scrutinee() {
    // Without a schema nothing is known about the enum.
    let report = validate_with(None, "  :case anything\n    -> END\n");
    assert!(report.is_ok(), "{report:?}");

    // An unknown scrutinee and a boolean one are reported by their own checks.
    let schema = mood_schema();
    for scrutinee in ["missing()", "ready()"] {
        let source =
            format!(":: start default\n:match {scrutinee}\n  :case anything\n    -> END\n-> END\n");
        let files = [lower("dialogue/start.recite", &source)];
        let report = validate_inputs(
            files
                .iter()
                .map(recite_compiler::validation::ValidationInput::all_complete),
            Some(&schema),
            recite_compiler::validation::ProjectCompleteness::Complete,
        );
        assert!(
            report.diagnostics.iter().all(|diagnostic| !matches!(
                diagnostic.code.as_str(),
                "RECITE_VALIDATE054" | "RECITE_VALIDATE055"
            )),
            "{scrutinee}: {report:?}"
        );
    }
}
