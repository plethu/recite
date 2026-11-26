//! Match variant diagnostics in the authoring kernel: the suggestion is
//! applicable, and recovery never reports a dropped arm as missing.

use recite_core::schema::{ProjectSchema, load_schema_manifest_str};

use super::*;

fn mood_schema() -> ProjectSchema {
    load_schema_manifest_str(
        "schema/mood.schema.json",
        r#"{
  "schema_version": 1,
  "types": {"mood_kind": {"kind": "enum", "values": ["calm", "tired", "wary"]}},
  "conditions": {"mood": {"params": [], "returns": "enum:mood_kind"}}
}"#,
    )
    .schema
    .expect("valid schema")
}

fn kernel_with(source: &str) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::with_schema(mood_schema()).expect("schema accepted");
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("source accepted");
    kernel
}

fn codes(kernel: &AuthoringKernel) -> Vec<String> {
    kernel
        .snapshot()
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.as_str().to_owned())
        .collect()
}

const MATCH: &str = concat!(
    ":: start default\n",
    ":match mood()\n",
    "  :case tird\n",
    "    -> END\n",
    "  :case _\n",
    "    -> END\n",
);

#[test]
fn an_undeclared_variant_suggestion_replaces_only_the_variant() {
    let kernel = kernel_with(MATCH);
    assert_eq!(codes(&kernel), ["RECITE_VALIDATE054"]);
    let plan = kernel
        .snapshot()
        .plan_apply_suggestion_in_range(&key("main.recite"), SourceRange::point(position(3, 9)))
        .expect("the variant has a suggestion");
    let applied = apply_plan(&plan, &saved_sources(&[("main.recite", MATCH)]));
    assert_eq!(
        applied.get(&key("main.recite")).map(String::as_str),
        Some(MATCH.replace(":case tird", ":case tired").as_str())
    );
}

#[test]
fn a_dropped_arm_is_not_reported_as_an_uncovered_variant() {
    // `:case calm extra` is malformed and dropped; the match must not then
    // claim `calm` has no arm.
    let kernel = kernel_with(concat!(
        ":: start default\n",
        ":match mood()\n",
        "  :case calm extra\n",
        "    -> END\n",
        "  :case tired\n",
        "    -> END\n",
        "  :case wary\n",
        "    -> END\n",
    ));
    let codes = codes(&kernel);
    assert!(!codes.is_empty(), "the malformed arm is reported");
    assert!(
        codes
            .iter()
            .all(|code| code != "RECITE_VALIDATE054" && code != "RECITE_VALIDATE055"),
        "{codes:?}"
    );
}

#[test]
fn removing_an_arm_in_an_edit_reports_the_uncovered_variant() {
    let complete = concat!(
        ":: start default\n",
        ":match mood()\n",
        "  :case calm\n",
        "    -> END\n",
        "  :case tired\n",
        "    -> END\n",
        "  :case wary\n",
        "    -> END\n",
    );
    let mut kernel = kernel_with(complete);
    assert!(codes(&kernel).is_empty(), "{:?}", codes(&kernel));
    let generation = kernel.snapshot().generation();
    kernel
        .apply(AuthoringRequest::new(
            generation,
            [SavedDocument::new(
                key("main.recite"),
                complete.replace("  :case wary\n    -> END\n", ""),
            )],
            [],
        ))
        .expect("edit accepted");
    assert_eq!(codes(&kernel), ["RECITE_VALIDATE055"]);
}

#[test]
fn a_one_character_variant_suggestion_is_also_applicable() {
    let source = MATCH.replace(":case tird", ":case w");
    let mut kernel = AuthoringKernel::with_schema(
        load_schema_manifest_str(
            "schema/arrow.schema.json",
            r#"{
  "schema_version": 1,
  "types": {"mood_kind": {"kind": "enum", "values": ["wa", "calm"]}},
  "conditions": {"mood": {"params": [], "returns": "enum:mood_kind"}}
}"#,
        )
        .schema
        .expect("valid schema"),
    )
    .expect("schema accepted");
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source.as_str())],
            [],
        ))
        .expect("source accepted");
    assert_eq!(codes(&kernel), ["RECITE_VALIDATE054"]);
    let plan = kernel
        .snapshot()
        .plan_apply_suggestion_in_range(&key("main.recite"), SourceRange::point(position(3, 9)))
        .expect("a one-character variant is applicable");
    let applied = apply_plan(&plan, &saved_sources(&[("main.recite", source.as_str())]));
    assert_eq!(
        applied.get(&key("main.recite")).map(String::as_str),
        Some(MATCH.replace(":case tird", ":case wa").as_str())
    );
}
