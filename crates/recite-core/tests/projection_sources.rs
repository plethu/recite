use recite_core::schema::{
    MetadataOccurrence, SchemaProjectionInputSource, load_schema_manifest_str,
};

fn manifest(metadata: &str, selector: &str, input_source: &str, type_ref: &str) -> String {
    format!(
        r#"{{
      "schema_version": 1,
      "metadata": {metadata},
      "availability_reasons": {{"blocked": {{"template": "{{value}}", "params": [{{"name": "value", "type": "int"}}]}}}},
      "presentation_projectors": {{"p": {{
        "candidates": {selector},
        "inputs": [{{"name": "value", "source": {input_source}, "type": "{type_ref}"}}],
        "queries": {{}}, "outputs": {{}}
      }}}}
    }}"#
    )
}

fn diagnostic_ids(report: &recite_core::schema::SchemaLoadReport) -> Vec<&str> {
    report
        .diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.presentation.as_ref())
        .map(|presentation| presentation.id().as_str())
        .collect()
}

#[test]
fn metadata_cardinality_and_input_types_accept_exactly_the_compatible_shapes() {
    let allowed = [
        (false, "only", "int"),
        (true, "only", "int"),
        (true, "first", "int"),
        (true, "last", "int"),
        (true, "index", "int"),
        (true, "all", "array:int"),
    ];
    for repeatable in [false, true] {
        for (name, token, occurrence) in [
            ("only", "\"only\"", MetadataOccurrence::Only),
            ("first", "\"first\"", MetadataOccurrence::First),
            ("last", "\"last\"", MetadataOccurrence::Last),
            ("all", "\"all\"", MetadataOccurrence::All),
            ("index", "{\"index\":2}", MetadataOccurrence::Index(2)),
        ] {
            for type_ref in ["int", "float", "array:int", "array:float"] {
                let metadata = format!(
                    r#"{{"tag": {{"targets": ["choice"], "type": "int", "repeatable": {repeatable}}}}}"#
                );
                let selector = r#"{"kind": "metadata_key", "target": "choice", "key": "tag"}"#;
                let source = format!(
                    r#"{{"kind": "candidate_metadata", "key": "tag", "occurrence": {token}}}"#
                );
                let report = load_schema_manifest_str(
                    "occurrence.json",
                    &manifest(&metadata, selector, &source, type_ref),
                );
                let valid = allowed.contains(&(repeatable, name, type_ref));
                assert_eq!(
                    report.schema.is_some(),
                    valid,
                    "{repeatable}/{name}/{type_ref}"
                );
                assert_eq!(report.diagnostics.is_empty(), valid);
                assert!(
                    report
                        .diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.record().is_ok())
                );
                if let Some(schema) = report.schema {
                    assert_eq!(
                        schema.presentation_projectors["p"].inputs[0].source,
                        SchemaProjectionInputSource::CandidateMetadata {
                            key: "tag".to_owned(),
                            occurrence: occurrence.clone()
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn unsupported_metadata_occurrence_is_a_structured_schema_failure() {
    let metadata = r#"{"tag": {"targets": ["choice"], "type": "int"}}"#;
    let selector = r#"{"kind": "metadata_key", "target": "choice", "key": "tag"}"#;
    let source = r#"{"kind": "candidate_metadata", "key": "tag", "occurrence": "unknown"}"#;
    let report = load_schema_manifest_str(
        "occurrence.json",
        &manifest(metadata, selector, source, "int"),
    );
    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_ids(&report),
        ["diagnostic-schema-001-projection-occurrence"]
    );
    assert!(report.diagnostics[0].record().is_ok());
}

#[test]
fn candidate_id_sources_match_the_selected_metadata_target() {
    for target in ["line", "choice", "block", "project"] {
        let metadata = format!(r#"{{"tag": {{"targets": ["{target}"], "type": "string"}}}}"#);
        let selector = format!(r#"{{"kind": "metadata_key", "target": "{target}", "key": "tag"}}"#);
        for (kind, owner) in [
            ("candidate_line_id", "line"),
            ("candidate_choice_id", "choice"),
            ("candidate_block_id", "block"),
            ("candidate_project", "project"),
            ("candidate_effect_request_id", "effect"),
        ] {
            let source = format!(r#"{{"kind": "{kind}"}}"#);
            let report = load_schema_manifest_str(
                "candidate.json",
                &manifest(&metadata, &selector, &source, "string"),
            );
            if owner == target {
                assert!(
                    report.diagnostics.is_empty(),
                    "{target}/{kind}: {:?}",
                    report.diagnostics
                );
                assert!(report.schema.is_some());
            } else {
                assert_eq!(
                    diagnostic_ids(&report),
                    ["diagnostic-schema-001-projection-candidate-source"],
                    "{target}/{kind}"
                );
                assert!(report.schema.is_none());
            }
        }
    }
}

#[test]
fn candidate_metadata_and_reason_sources_require_their_own_selector_and_type() {
    for (selector, source, type_ref, expected) in [
        (
            r#"{"kind":"runtime_event","event":"dialogue"}"#,
            r#"{"kind":"candidate_effect_request_id"}"#,
            "string",
            "diagnostic-schema-001-projection-candidate-source",
        ),
        (
            r#"{"kind":"runtime_event","event":"effect"}"#,
            r#"{"kind":"candidate_effect_request_id"}"#,
            "string",
            "",
        ),
        (
            r#"{"kind":"runtime_event","event":"dialogue"}"#,
            r#"{"kind":"candidate_metadata","key":"tag"}"#,
            "int",
            "diagnostic-schema-001-projection-candidate-no-target",
        ),
        (
            r#"{"kind":"runtime_event","event":"dialogue"}"#,
            r#"{"kind":"availability_reason_arg","name":"value"}"#,
            "int",
            "diagnostic-schema-001-projection-reason-no-selector",
        ),
        (
            r#"{"kind":"availability_reason","reason":"blocked"}"#,
            r#"{"kind":"availability_reason_arg","name":"missing"}"#,
            "int",
            "diagnostic-schema-004-projection-reason-arg",
        ),
        (
            r#"{"kind":"availability_reason","reason":"blocked"}"#,
            r#"{"kind":"availability_reason_arg","name":"value"}"#,
            "string",
            "diagnostic-schema-001-projection-reason-type",
        ),
        (
            r#"{"kind":"availability_reason","reason":"blocked"}"#,
            r#"{"kind":"availability_reason_arg","name":"value"}"#,
            "int",
            "",
        ),
        (
            r#"{"kind":"availability_reason","reason":"blocked"}"#,
            r#"{"kind":"candidate_line_id"}"#,
            "string",
            "diagnostic-schema-001-projection-candidate-source",
        ),
    ] {
        let report =
            load_schema_manifest_str("source.json", &manifest("{}", selector, source, type_ref));
        if expected.is_empty() {
            assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
            assert!(report.schema.is_some());
        } else {
            assert_eq!(diagnostic_ids(&report), [expected]);
            assert!(report.schema.is_none());
            assert!(
                report
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.record().is_ok())
            );
        }
    }
}
