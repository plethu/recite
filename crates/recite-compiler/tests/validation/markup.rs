use std::collections::BTreeMap;

use recite_core::schema::{MarkupDefinition, ProjectSchema};

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
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @c1qdb71\n",
            "  [slow]Hello [em]there[/em][/slow]\n",
            "  ? @ask\n",
            "    [pause]Ask now.\n",
            "    -> END\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert!(report.is_ok(), "valid markup should pass: {report:?}");
}

#[test]
fn reports_unknown_inline_markup_tags_on_tag_names() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @e343ftd\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE022", "RECITE_VALIDATE022"]);
    assert_spans(&report, [(3, 4), (3, 17)]);
}

#[test]
fn reports_multiline_inline_markup_spans_at_author_visible_columns() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @kwk8czf\n",
            "  First line.\n",
            "  [ghost]Second line.\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE022"]);
    assert_spans(&report, [(4, 4)]);
}

#[test]
fn reports_missing_required_inline_markup_closing_tag_on_opening_name() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "> @ffee6p8\n", "  [slow]Hello\n",),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE024"]);
    assert_spans(&report, [(3, 4)]);
}

#[test]
fn reports_unexpected_inline_markup_closing_tag_on_closing_name() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(":: start default\n", "> @ffee6p8\n", "  Hello[/slow]\n",),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 10)]);
}

#[test]
fn reports_mismatched_inline_markup_closing_tag_on_closing_name() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @vksazm3\n",
            "  [slow]Hello[/shake][/slow]\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 16)]);
}

#[test]
fn reports_nested_inline_markup_inside_non_nesting_tag() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @z9vg1gf\n",
            "  [shake]Hello [slow]there[/slow][/shake]\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE025"]);
    assert_spans(&report, [(3, 17)]);
}

#[test]
fn reports_closing_tag_for_standalone_inline_markup() {
    let schema = markup_schema();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @jws2zpf\n",
            "  [pause]Wait[/pause]\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE023"]);
    assert_spans(&report, [(3, 16)]);
}

#[test]
fn skips_inline_markup_validation_without_schema_definitions() {
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @r7pbbb5\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(report.is_ok(), "schema-less markup should pass: {report:?}");
}

#[test]
fn explicit_schema_without_markup_definitions_reports_unknown_tags() {
    let schema = ProjectSchema::empty_v1();
    let files = [lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @hmnnx0e\n",
            "  [ghost]Hello[/ghost]\n",
        ),
    )];

    let report = validate_inputs(
        files
            .iter()
            .map(recite_compiler::validation::ValidationInput::all_complete),
        Some(&schema),
        recite_compiler::validation::ProjectCompleteness::Complete,
    );

    assert_codes(&report, ["RECITE_VALIDATE022", "RECITE_VALIDATE022"]);
    assert_spans(&report, [(3, 4), (3, 17)]);
}
