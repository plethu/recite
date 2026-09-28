#![cfg(test)]

use recite_import::{FieldMapping, ImportRequest, ImportStatus, SourceFamily, import};

#[test]
fn end_is_a_real_node_name_in_twee_and_yarn() {
    for (family, source) in [
        (
            SourceFamily::Twee,
            ":: Start\nHello.\n[[Continue->END]]\n:: END\nStill here.\n",
        ),
        (
            SourceFamily::Yarn,
            "title: Start\n---\nHello.\n<<jump END>>\n===\ntitle: END\n---\nStill here.\n===\n",
        ),
    ] {
        let report = import(ImportRequest {
            family,
            file: "input",
            source,
            mapping: None,
            schema: None,
        })
        .expect("report");
        assert_eq!(report.status, ImportStatus::Complete);
        let end = report
            .mappings
            .iter()
            .find(|mapping| {
                mapping.construct == "block" && mapping.original_id.as_deref() == Some("END")
            })
            .expect("END mapping");
        assert!(report.source.contains(&format!("-> {}", end.generated_id)));
    }
}

#[test]
fn story_format_markup_and_repeated_control_flow_do_not_pass_as_plain_text() {
    for body in [
        "''Emphasis''",
        "@@color:red;Hello@@",
        "//italic//",
        "{(print: 1)}",
        "<script>work()</script>",
        "[[Next]]\nAfter the choice.",
    ] {
        let source = format!(":: Start\nHello.\n{body}\n");
        let report = import(ImportRequest {
            family: SourceFamily::Twee,
            file: "input",
            source: &source,
            mapping: None,
            schema: None,
        })
        .expect("report");
        assert_eq!(report.status, ImportStatus::Invalid, "{body}");
        assert!(!report.source.contains("Hello."));
    }
}

#[test]
fn schema_backed_import_preserves_native_speaker_policy() {
    let schema = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        r#"{"schema_version":1,"speakers":{"operator":{"display_name":"Operator"}}}"#,
    )
    .schema
    .expect("schema");
    let mapping: FieldMapping =
        serde_json::from_str(r#"{"block":"node","text":"text","speaker":"speaker"}"#)
            .expect("mapping");
    let report = import(ImportRequest {
        family: SourceFamily::Json,
        file: "input.json",
        source: r#"[{"node":"start","text":"Hello.","speaker":"unknown"}]"#,
        mapping: Some(&mapping),
        schema: Some(&schema),
    })
    .expect("report");
    // Native line speakers are not constrained to the schema's speaker registry;
    // the importer must not invent an additional compatibility rule.
    assert_eq!(report.status, ImportStatus::Complete);
    assert!(report.native_diagnostics.is_empty());
    assert!(report.source.contains("speaker=unknown"));
}

#[test]
fn accepted_source_ids_are_preserved_and_other_ids_have_explicit_mappings() {
    let mapping: FieldMapping =
        serde_json::from_str(r#"{"block":"node","text":"text","id":"id"}"#).expect("mapping");
    let source = r#"[{"node":"a","text":"First.","id":"authored@11111111111111111111"},{"node":"a","text":"Second.","id":"legacy-key"}]"#;
    let report = import(ImportRequest {
        family: SourceFamily::Json,
        file: "input.json",
        source,
        mapping: Some(&mapping),
        schema: None,
    })
    .expect("report");
    assert_eq!(report.status, ImportStatus::Complete);
    assert!(report.source.contains("> authored@11111111111111111111"));
    let generated = report
        .mappings
        .iter()
        .find(|mapping| mapping.original_id.as_deref() == Some("legacy-key"))
        .expect("mapping");
    assert!(report.source.contains(&generated.generated_id));
    assert!(generated.generated_id.contains('@'));
}
