use recite_core::load_schema_source_str;

use super::schema_source_diagnostic_support::{assert_presentation_by_id, string};
use crate::assert_recordable_diagnostics;

#[test]
fn producer_is_mandatory_and_generated_fields_are_rejected() {
    let missing_id = load_schema_source_str("schema.toml", "schema_version = 1\n\n[producer]\n\n");
    assert!(missing_id.source.is_none());
    assert_presentation_by_id(
        &missing_id,
        "diagnostic-schema-001-source-producer-id-required",
        [],
    );
    assert_recordable_diagnostics(&missing_id);

    let generated = load_schema_source_str(
        "schema.toml",
        "schema_version = 1\ncontent_fingerprint = \"generated\"\n\n[producer]\nid = \"dialogue\"\n",
    );
    assert!(generated.source.is_none());
    assert_presentation_by_id(
        &generated,
        "diagnostic-schema-001-source-generated-field",
        [("key", string("content_fingerprint"))],
    );
    assert_recordable_diagnostics(&generated);

    let string_version = load_schema_source_str(
        "schema.toml",
        "schema_version = \"1\"\n\n[producer]\nid = \"dialogue\"\n",
    );
    assert!(string_version.source.is_none());
    assert_presentation_by_id(
        &string_version,
        "diagnostic-schema-001-schema-version-type",
        [],
    );
    assert_recordable_diagnostics(&string_version);
}

#[test]
fn non_finite_toml_numbers_are_rejected_before_lowering() {
    for token in ["nan", "inf", "-inf"] {
        let source = format!(
            "schema_version = 1\n[producer]\nid = \"dialogue\"\n[availability_reasons.reason]\ntemplate = \"{{value}}\"\nparams = [{{ name = \"value\", type = \"float\" }}]\n[conditions.ready]\nreturns = \"bool\"\n[conditions.ready.availability_reason]\nreason = \"reason\"\n[conditions.ready.availability_reason.args.value]\nvalue = {token}\n"
        );
        let report = load_schema_source_str("non-finite.toml", &source);
        assert!(report.source.is_none(), "{token}");
        let diagnostic =
            assert_presentation_by_id(&report, "diagnostic-schema-001-source-non-finite", []);
        assert_eq!(diagnostic.span.start.line(), 12, "{token}");
        assert_recordable_diagnostics(&report);
    }
}

#[test]
fn canonical_semantic_validation_is_shared_for_source_toml() {
    let source = r#"
schema_version = 1
[producer]
id = "dialogue"

[conditions.ready]
params = [{ name = "actor", type = "missing_type" }]
returns = "bool"
"#;
    let report = load_schema_source_str("invalid.toml", source);
    assert!(report.source.is_none());
    assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-invalid-parameter-type",
        [
            ("parameter", string("actor")),
            ("type_ref", string("missing_type")),
        ],
    );
    assert_recordable_diagnostics(&report);
}

#[test]
fn condition_type_diagnostics_follow_field_and_parameter_paths() {
    let source = r#"schema_version = 1
[producer]
id = "dialogue"

[conditions.ready]
returns = "enum:missing" # same text in a comment: enum:missing
params = [{ name = "p", type = "enum:missing" }]
"#;
    let report = load_schema_source_str("condition-paths.toml", source);
    assert!(report.source.is_none());

    let return_diagnostic = assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-unknown-enum",
        [
            ("name", string("missing")),
            ("owner", string("condition 'ready' return type")),
        ],
    );
    let parameter_diagnostic = assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-unknown-enum",
        [
            ("name", string("missing")),
            ("owner", string("condition 'ready' parameter 'p'")),
        ],
    );
    assert_eq!(return_diagnostic.span.start.line(), 6);
    assert_eq!(parameter_diagnostic.span.start.line(), 7);
    assert_ne!(
        return_diagnostic.span.start.column(),
        parameter_diagnostic.span.start.column()
    );
    assert_recordable_diagnostics(&report);
}

#[test]
fn parameter_path_spans_cover_quoted_inline_and_array_tables() {
    let source = r#"schema_version = 1
[producer]
id = "dialogue"

[conditions."inline"]
returns = "enum:missing" # enum:missing in an unrelated comment
params = [{ name = "p", type = "enum:missing" }]

[conditions."array"]
returns = "bool"
[[conditions.array.params]]
name = "p"
type = "enum:missing"
"#;
    let report = load_schema_source_str("parameter-paths.toml", source);
    assert!(report.source.is_none());
    assert_eq!(report.diagnostics.len(), 3);
    let inline = assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-unknown-enum",
        [
            ("name", string("missing")),
            ("owner", string("condition 'inline' parameter 'p'")),
        ],
    );
    let array = assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-unknown-enum",
        [
            ("name", string("missing")),
            ("owner", string("condition 'array' parameter 'p'")),
        ],
    );
    assert_eq!(inline.span.start.line(), 7);
    assert_eq!(array.span.start.line(), 13);
    assert_recordable_diagnostics(&report);
}
