#![cfg(test)]

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use recite_import::{
    FieldMapping, ImportReport, ImportRequest, ImportStatus, SourceFamily, import,
};

fn run(
    family: SourceFamily,
    file: &str,
    source: &str,
    mapping: Option<&FieldMapping>,
) -> ImportReport {
    import(ImportRequest {
        family,
        file,
        source,
        mapping,
        schema: None,
        default_block: true,
    })
    .expect("import contract")
}

#[test]
fn complete_fixtures_have_deterministic_source_and_provenance() {
    let mapping = serde_json::from_str(include_str!("../../../fixtures/import/mapping.json"))
        .expect("mapping");
    for (family, name, source) in [
        (
            SourceFamily::Json,
            "lines.json",
            include_str!("../../../fixtures/import/lines.json"),
        ),
        (
            SourceFamily::Csv,
            "lines.csv",
            include_str!("../../../fixtures/import/lines.csv"),
        ),
        (
            SourceFamily::Twee,
            "passages.twee",
            include_str!("../../../fixtures/import/passages.twee"),
        ),
        (
            SourceFamily::Ink,
            "knots.ink",
            include_str!("../../../fixtures/import/knots.ink"),
        ),
        (
            SourceFamily::Yarn,
            "nodes.yarn",
            include_str!("../../../fixtures/import/nodes.yarn"),
        ),
    ] {
        let mapping = matches!(family, SourceFamily::Json | SourceFamily::Csv).then_some(&mapping);
        let report = run(family, name, source, mapping);
        assert_eq!(report.status, ImportStatus::Complete, "{name}: {report:#?}");
        assert_eq!(report, run(family, name, source, mapping));
        assert!(report.source.contains("The signal returns."));
        assert!(
            report
                .mappings
                .iter()
                .all(|mapping| mapping.provenance.file == name)
        );
        insta::assert_json_snapshot!(name, report);
    }
}

#[test]
fn same_named_blocks_in_separate_imports_compile_with_one_default() {
    let source = ":: Start\nHello.\n[[Go->End]]\n:: End\nBye.\n";
    let first = import(ImportRequest {
        family: SourceFamily::Twee,
        file: "story/north/input.twee",
        source,
        mapping: None,
        schema: None,
        default_block: true,
    })
    .expect("first import");
    let second = import(ImportRequest {
        family: SourceFamily::Twee,
        file: "story/south/input.twee",
        source,
        mapping: None,
        schema: None,
        default_block: false,
    })
    .expect("second import");
    assert_eq!(first.status, ImportStatus::Complete);
    assert_eq!(second.status, ImportStatus::Complete);
    assert_eq!(first.source.matches(" default\n").count(), 1);
    assert_eq!(second.source.matches(" default\n").count(), 0);
    let first_blocks: Vec<_> = first
        .mappings
        .iter()
        .filter(|mapping| mapping.construct == "block")
        .map(|mapping| mapping.generated_id.as_str())
        .collect();
    let second_blocks: Vec<_> = second
        .mappings
        .iter()
        .filter(|mapping| mapping.construct == "block")
        .map(|mapping| mapping.generated_id.as_str())
        .collect();
    assert!(first_blocks.iter().all(|id| !second_blocks.contains(id)));
    for report in [&first, &second] {
        let end = report
            .mappings
            .iter()
            .find(|mapping| {
                mapping.construct == "block" && mapping.original_id.as_deref() == Some("End")
            })
            .expect("End block");
        assert!(report.source.contains(&format!("-> {}", end.generated_id)));
    }
    let compiled = compile_inputs(
        [
            CompileInput::new("north.recite", &first.source),
            CompileInput::new("south.recite", &second.source),
        ],
        CompileOptions::new(
            CompilerVersion::new("test").expect("version"),
            CompiledAssetId::new("migration-test").expect("asset"),
            SourceMapId::new("migration-test.map").expect("source map"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("native compile");
    assert!(compiled.is_ok(), "{:#?}", compiled.diagnostics);
}

#[test]
fn unmapped_fields_keep_precise_json_pointer_and_csv_header_locations() {
    let mapping = serde_json::from_str(r#"{"block":"node","text":"text"}"#).expect("mapping");
    let json = run(
        SourceFamily::Json,
        "input.json",
        r#"[{"node":"start","text":"Hello.","a/b~c":42}]"#,
        Some(&mapping),
    );
    assert_eq!(json.status, ImportStatus::Partial);
    assert_eq!(
        serde_json::to_value(&json.items[0].provenance.location).expect("location")["pointer"],
        "/0/a~1b~0c"
    );
    let csv = run(
        SourceFamily::Csv,
        "input.csv",
        "node,text,unused\nstart,Hello.,x\n",
        Some(&mapping),
    );
    assert_eq!(csv.status, ImportStatus::Partial);
    let location = serde_json::to_value(&csv.items[0].provenance.location).expect("location");
    assert_eq!(location["row"], 2);
    assert_eq!(location["column"], 3);
    assert_eq!(location["header"], "unused");
}

#[test]
fn duplicate_fields_ambiguous_mappings_and_malformed_records_are_invalid() {
    let mapping: FieldMapping =
        serde_json::from_str(r#"{"block":"node","text":"text"}"#).expect("mapping");
    for source in [
        r#"[{"node":"a","node":"b","text":"Hello."}]"#,
        r#"[{"node":"a","text":3}]"#,
        "{}",
        "[",
        r#"[{"node":"a"}]"#,
    ] {
        let report = run(SourceFamily::Json, "bad.json", source, Some(&mapping));
        assert_eq!(report.status, ImportStatus::Invalid, "{source}");
        assert!(!report.items.is_empty());
    }
    let bad_mapping = serde_json::from_str(r#"{"block":"same","text":"same"}"#).expect("mapping");
    assert_eq!(
        run(SourceFamily::Json, "bad.json", "[]", Some(&bad_mapping)).status,
        ImportStatus::Invalid
    );
    for source in [
        "node,text,text\na,b,c",
        "node,text\na,b,c",
        "node,text\na",
        "",
    ] {
        assert_eq!(
            run(SourceFamily::Csv, "bad.csv", source, Some(&mapping)).status,
            ImportStatus::Invalid
        );
    }
}

#[test]
fn native_validation_rejects_unknown_targets_and_duplicate_authored_ids() {
    let mapping = serde_json::from_str(include_str!("../../../fixtures/import/mapping.json"))
        .expect("mapping");
    for source in [
        include_str!("../../../fixtures/import/lines.json")
            .replace("\"End\",\"text\"", "\"Missing\",\"text\""),
        include_str!("../../../fixtures/import/lines.json")
            .replace("11111111111111111112", "11111111111111111111"),
    ] {
        let report = run(SourceFamily::Json, "bad.json", &source, Some(&mapping));
        assert_eq!(report.status, ImportStatus::Invalid);
        assert!(!report.native_diagnostics.is_empty());
        assert!(
            !report.source.is_empty(),
            "invalid source remains inspectable"
        );
    }
}

#[test]
fn conditional_blocks_are_held_back_instead_of_emitting_unconditional_text() {
    for (family, source) in [
        (
            SourceFamily::Ink,
            "=== safe ===\nHello.\n-> END\n=== unsafe ===\n{flag:\nSecret.\n}\n",
        ),
        (
            SourceFamily::Yarn,
            "title: safe\n---\nHello.\n===\ntitle: unsafe\n---\n<<if $flag>>\nSecret.\n<<endif>>\n===\n",
        ),
        (
            SourceFamily::Twee,
            ":: safe\nHello.\n:: unsafe\n<<if $flag>>\nSecret.\n<</if>>\n",
        ),
    ] {
        let report = run(family, "input", source, None);
        assert_eq!(report.status, ImportStatus::Partial);
        assert!(report.source.contains("Hello."));
        assert!(!report.source.contains("Secret."));
        assert!(
            report
                .items
                .iter()
                .any(|item| item.construct == "held_back")
        );
        insta::assert_json_snapshot!(format!("partial_{family:?}"), report);
    }
}

#[test]
fn unsupported_yarn_headers_and_truncated_nodes_do_not_become_dialogue() {
    let report = run(
        SourceFamily::Yarn,
        "input.yarn",
        "title: a\ntags: hidden\n---\nHello.\n===\n",
        None,
    );
    assert_eq!(report.status, ImportStatus::Invalid);
    assert!(!report.source.contains("tags:"));
    assert_eq!(
        run(
            SourceFamily::Yarn,
            "input.yarn",
            "title: a\n---\nHello.",
            None
        )
        .status,
        ImportStatus::Invalid
    );
}

#[test]
fn migration_never_turns_record_text_into_native_statements() {
    let mapping = serde_json::from_str(r#"{"block":"node","text":"text"}"#).expect("mapping");
    for text in [
        "! immediate steal()",
        "> other@11111111111111111111",
        ":: injected",
        "Hello.\n-> END",
        "{variable}",
    ] {
        let source = serde_json::json!([{"node":"start","text":text}]).to_string();
        let report = run(SourceFamily::Json, "input.json", &source, Some(&mapping));
        assert_ne!(report.status, ImportStatus::Complete);
        assert!(!report.source.contains(text));
    }
}
