use std::collections::BTreeMap;

use recite_core::{MarkupDefinition, ProjectSchema};

use super::*;

fn markup_schema() -> ProjectSchema {
    let mut schema = ProjectSchema::empty_v1();
    schema.markup = BTreeMap::from([
        (
            "em".to_owned(),
            MarkupDefinition {
                requires_closing: true,
                translatable: true,
                allows_nesting: true,
            },
        ),
        (
            "pause".to_owned(),
            MarkupDefinition {
                requires_closing: false,
                translatable: false,
                allows_nesting: true,
            },
        ),
        (
            "shake".to_owned(),
            MarkupDefinition {
                requires_closing: true,
                translatable: true,
                allows_nesting: false,
            },
        ),
        (
            "slow".to_owned(),
            MarkupDefinition {
                requires_closing: true,
                translatable: true,
                allows_nesting: true,
            },
        ),
    ]);
    schema
}

#[test]
fn accepts_schema_declared_inline_markup_on_lines_and_choices() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@099f908852bbbcc90296\n",
            "  [slow]Hello [em]there[/em][/slow]\n",
            "  ? ask@40f78a1bf92a5059ac4c\n",
            "    [pause]Ask now.\n",
            "    -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "valid markup should pass: {report:?}");
}

#[test]
fn reports_unknown_inline_markup_tags_on_tag_names() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@7ed07c3e34c0ca877555\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE022", "RECITE_VALIDATE022"]);
    assert_spans(&report, [(3, 4), (3, 17)]);
}

#[test]
fn reports_multiline_inline_markup_spans_at_author_visible_columns() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@684c457f63157546ac21\n",
            "  First line.\n",
            "  [ghost]Second line.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE022"]);
    assert_spans(&report, [(4, 4)]);
}

#[test]
fn reports_missing_required_inline_markup_closing_tag_on_opening_name() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@11111111111111111111\n",
            "  [slow]Hello\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE024"]);
    assert_spans(&report, [(3, 4)]);
}

#[test]
fn reports_unexpected_inline_markup_closing_tag_on_closing_name() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@11111111111111111111\n",
            "  Hello[/slow]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 10)]);
}

#[test]
fn reports_mismatched_inline_markup_closing_tag_on_closing_name() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@264be2851d3c4e5aece6\n",
            "  [slow]Hello[/shake][/slow]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 16)]);
}

#[test]
fn reports_nested_inline_markup_inside_non_nesting_tag() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@16d57c9a758d714a1cf5\n",
            "  [shake]Hello [slow]there[/slow][/shake]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE025"]);
    assert_spans(&report, [(3, 17)]);
}

#[test]
fn reports_closing_tag_for_standalone_inline_markup() {
    let schema = markup_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@99f1284aca47a1ccfa25\n",
            "  [pause]Wait[/pause]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 16)]);
}

#[test]
fn skips_inline_markup_validation_without_schema_definitions() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@1312dfa7366087564ca0\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(report.is_ok(), "schema-less markup should pass: {report:?}");
}

#[test]
fn explicit_schema_without_markup_definitions_reports_unknown_tags() {
    let schema = ProjectSchema::empty_v1();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@d6f282ffcc0a15e67dd6\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE022", "RECITE_VALIDATE022"]);
    assert_spans(&report, [(3, 4), (3, 17)]);
}
