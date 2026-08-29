use recite_core::load_schema_source_str;

use crate::assert_recordable_diagnostics;

use super::schema_source_diagnostic_support::{
    assert_empty_value_field, assert_presentation_by_id, string,
};

#[test]
fn provenance_and_fingerprint_spans_follow_reordered_cst_fields() {
    let source = r#"schema_version = 1
[producer]
id = "dialogue"

[registries.r]
values = ["item"]
producer_fingerprints = [
  { value = "", id = "", algorithm = "", kind = "" },
]

[registries.r.origin]
id = ""
kind = ""
bad = "extension"
"#;
    let report = load_schema_source_str("provenance-paths.toml", source);
    assert!(report.source.is_none());

    for (field, line) in [
        ("registry 'r' origin id", 12),
        ("registry 'r' origin kind", 13),
        ("registry 'r' producer fingerprint id", 8),
        ("registry 'r' producer fingerprint kind", 8),
        ("registry 'r' producer fingerprint algorithm", 8),
        ("registry 'r' producer fingerprint value", 8),
    ] {
        assert_empty_value_field(&report, field, line);
    }
    let extension = assert_presentation_by_id(
        &report,
        "diagnostic-schema-001-origin-extension",
        [("owner", string("registry 'r'")), ("key", string("bad"))],
    );
    assert_eq!(extension.span.start.line(), 14);
    assert_recordable_diagnostics(&report);
}

#[test]
fn domain_provenance_spans_follow_context_and_value_paths() {
    let source = r#"schema_version = 1
[producer]
id = "dialogue"

[metadata_domains.tone]
kind = "flat"
values = ["calm"]
[metadata_domains.tone.origin]
id = ""
kind = ""
[metadata_domains.tone.value_origins.calm]
id = ""
kind = ""

[metadata_domains.tone_by_actor]
kind = "contextual"
selector = "field:speaker"
values_by_context = { rhea = ["calm"] }
missing_context = { policy = "fallback", domain = "tone" }
[metadata_domains.tone_by_actor.context_origins.rhea]
id = ""
kind = ""
[metadata_domains.tone_by_actor.value_origins.rhea.calm]
id = ""
kind = ""
"#;
    let report = load_schema_source_str("domain-provenance-paths.toml", source);
    assert!(report.source.is_none());
    for (field, line) in [
        ("metadata domain 'tone' origin id", 9),
        ("metadata domain 'tone' origin kind", 10),
        ("metadata domain 'tone' value 'calm' origin id", 12),
        ("metadata domain 'tone' value 'calm' origin kind", 13),
        (
            "metadata domain 'tone_by_actor' context 'rhea' origin id",
            21,
        ),
        (
            "metadata domain 'tone_by_actor' context 'rhea' origin kind",
            22,
        ),
        (
            "metadata domain 'tone_by_actor' context value 'calm' origin id",
            24,
        ),
        (
            "metadata domain 'tone_by_actor' context value 'calm' origin kind",
            25,
        ),
    ] {
        assert_empty_value_field(&report, field, line);
    }
    assert_recordable_diagnostics(&report);
}

#[test]
fn source_diagnostics_point_at_commented_headers_and_multiline_values() {
    let source = r#"
schema_version = 1

[producer] # this header comment is not part of the table
id = "dialogue"

[types."bad key"] # quoted dotted-key components are preserved
kind = "enum"
values = ["known"]
"#;
    let report = load_schema_source_str("spans.toml", source);
    let diagnostic = assert_presentation_by_id(
        &report,
        "diagnostic-schema-001-invalid-name",
        [("field", string("type name"))],
    );
    assert_eq!(diagnostic.span.start.line(), 7);
    assert!(diagnostic.span.end.is_some());

    let multiline = r#"
schema_version = 1
[producer]
id = "dialogue"
[speakers.actor]
[availability_reasons.not_ready]
template = """
{speaker} is not ready
"""
params = [{ name = "speaker", type = "speaker" }]
[conditions.ready]
params = [{ name = "actor", type = "speaker" }]
returns = "bool"
[conditions.ready.availability_reason]
reason = "not_ready"
[conditions.ready.availability_reason.args.speaker]
kind = "literal"
value = """missing"""
"#;
    let report = load_schema_source_str("multiline.toml", multiline);
    assert!(report.source.is_none());
    assert_presentation_by_id(
        &report,
        "diagnostic-schema-001-availability-binding-unknown-value",
        [
            ("condition", string("ready")),
            ("argument", string("speaker")),
            ("expected", string("speaker")),
            ("value", string("missing")),
        ],
    );
    assert_recordable_diagnostics(&report);
}

#[test]
fn projection_references_and_literal_domains_use_canonical_validation() {
    let valid = r#"
schema_version = 1
[producer]
id = "dialogue"

[metadata.skill]
targets = ["choice"]
type = "string"

[projection_queries.actor_skill]
params = [{ name = "skill", type = "string" }]
returns = "int"
max_calls_per_event = 1

[presentation_projectors.choice_skill]
candidates = { kind = "metadata_key", target = "choice", key = "skill" }
inputs = [
  { name = "skill", source = { kind = "candidate_metadata", key = "skill" }, type = "string", required = true },
]

[presentation_projectors.choice_skill.queries.current]
function = "actor_skill"
args = [{ input = "skill" }]

[presentation_projectors.choice_skill.outputs.badge]
target = "candidate"
kind = "badge"
slot = "prefix"
"#;
    let report = load_schema_source_str("projection.toml", valid);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let source = report.source.expect("valid projection");
    assert!(
        source
            .schema()
            .presentation_projectors
            .contains_key("choice_skill")
    );

    let invalid = valid.replace("function = \"actor_skill\"", "function = \"missing_query\"");
    let report = load_schema_source_str("invalid-projection.toml", &invalid);
    assert!(report.source.is_none());
    assert_presentation_by_id(
        &report,
        "diagnostic-schema-004-unknown-query-function",
        [
            ("projector", string("choice_skill")),
            ("query", string("current")),
            ("function", string("missing_query")),
        ],
    );
    assert_recordable_diagnostics(&report);
}
