#![cfg(test)]

use recite_import::{
    Action, FieldMapping, ImportRequest, ImportStatus, Location, SourceFamily, import,
};

#[test]
fn text_whitespace_loss_keeps_provenance_and_requires_review() {
    for (family, source, row) in [
        (SourceFamily::Twee, ":: Start\n  Hello.  \n", 2),
        (SourceFamily::Ink, "=== Start ===\n  Hello.  \n-> END\n", 2),
        (SourceFamily::Yarn, "title: Start\n---\nHello.  \n===\n", 3),
        (
            SourceFamily::Yarn,
            "title: Start\n---\nÉlodie: Hello.  \n===\n",
            3,
        ),
        (
            SourceFamily::Yarn,
            "title: Start\n---\nÉlodie: Hello.   #line:ffee6p8  \n===\n",
            3,
        ),
        (
            SourceFamily::Twee,
            ":: Start\nHello.\n[[ Again ->Start]]\n",
            3,
        ),
        (
            SourceFamily::Ink,
            "=== Start ===\nHello.\n+ [ Again ] -> Start\n",
            3,
        ),
    ] {
        let report = import(ImportRequest {
            family,
            file: "whitespace",
            source,
            mapping: None,
            schema: None,
            default_block: true,
            target_sources: None,
        })
        .expect("report");
        assert_eq!(
            report.status,
            ImportStatus::Partial,
            "{source}: {report:#?}"
        );
        assert_eq!(report.items.len(), 1, "{source}: {report:#?}");
        let item = &report.items[0];
        assert_eq!(item.action, recite_import::Action::ConvertedWithLoss);
        let item = serde_json::to_value(item).expect("item");
        assert_eq!(item["diagnostic"]["code"], "RECITE_IMPORT003");
        assert_eq!(item["provenance"]["file"], "whitespace");
        assert_eq!(item["provenance"]["location"]["span"]["start"]["line"], row);
        assert!(report.native_diagnostics.is_empty());
        assert!(report.source.contains("\n  Hello.\n"));
        if source.contains("#line:") {
            assert!(report.source.contains("@ffee6p8"));
        }
    }
}

#[test]
fn speaker_ids_survive_yarn_and_record_imports_as_structured_data() {
    let mapping: FieldMapping =
        serde_json::from_str(r#"{"block":"node","text":"text","speaker":"speaker"}"#)
            .expect("mapping");
    for speaker in ["Élodie", "张伟", "E\u{301}lodie", "élodie_2", "Jean-Luc"] {
        for (family, source, mapping) in [
            (
                SourceFamily::Yarn,
                format!("title: Start\n---\n{speaker}: Bonjour.\n===\n"),
                None,
            ),
            (
                SourceFamily::Json,
                serde_json::json!([{"node":"Start","text":"Bonjour.","speaker":speaker}])
                    .to_string(),
                Some(&mapping),
            ),
            (
                SourceFamily::Csv,
                format!("node,text,speaker\nStart,Bonjour.,{speaker}\n"),
                Some(&mapping),
            ),
        ] {
            let report = import(ImportRequest {
                family,
                file: "input",
                source: &source,
                mapping,
                schema: None,
                default_block: true,
                target_sources: None,
            })
            .expect("report");
            assert_eq!(report.status, ImportStatus::Complete, "{report:#?}");
            assert!(report.source.contains(&format!(" speaker={speaker} @")));
            assert!(report.source.contains("\n  Bonjour.\n"));
            assert!(!report.source.contains(&format!("{speaker}: Bonjour.")));
            assert!(report.native_diagnostics.is_empty());
        }
    }
}

#[test]
fn unsafe_speaker_prefixes_are_reported_instead_of_imported_as_prose() {
    for speaker in [
        "Élodie portrait=angry",
        "Élodie\"",
        "Élodie[",
        "Élodie(",
        "#tag",
        "-> Élodie",
        "Élodie//tag",
        "Élodie*",
    ] {
        let source = format!("title: Start\n---\n{speaker}: Bonjour.\n===\n");
        let report = import(ImportRequest {
            family: SourceFamily::Yarn,
            file: "input.yarn",
            source: &source,
            mapping: None,
            schema: None,
            default_block: true,
            target_sources: None,
        })
        .expect("report");
        assert_eq!(report.status, ImportStatus::Invalid, "{speaker}");
        assert!(!report.source.contains("Bonjour."));
        assert!(!report.items.is_empty());
    }
}

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
            default_block: true,
            target_sources: None,
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
            default_block: true,
            target_sources: None,
        })
        .expect("report");
        assert_eq!(report.status, ImportStatus::Invalid, "{body}");
        assert!(!report.source.contains("Hello."));
    }
}

fn speaker_import(mapping: &str, records: &str) -> recite_import::ImportReport {
    let schema = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        r#"{"schema_version":1,"speakers":{"operator":{"display_name":"Operator"}}}"#,
    )
    .schema
    .expect("schema");
    let mapping: FieldMapping = serde_json::from_str(mapping).expect("mapping");
    import(ImportRequest {
        family: SourceFamily::Json,
        file: "input.json",
        source: records,
        mapping: Some(&mapping),
        schema: Some(&schema),
        default_block: true,
        target_sources: None,
    })
    .expect("report")
}

#[test]
fn speakers_missing_from_the_schema_are_kept_for_review_once_each() {
    let report = speaker_import(
        r#"{"block":"node","text":"text","speaker":"speaker"}"#,
        r#"[{"node":"start","text":"Hello.","speaker":"operatr"},
            {"node":"start","text":"Again.","speaker":"operatr"},
            {"node":"start","text":"Who?","speaker":"stranger"},
            {"node":"start","text":"Fine.","speaker":"operator"}]"#,
    );
    // One unmapped name must not reject the whole import: the source keeps the speakers as
    // written, everything else still passes native validation, and each name is reported once.
    assert_eq!(report.status, ImportStatus::Partial);
    assert!(report.native_diagnostics.is_empty());
    assert!(report.source.contains("speaker=operatr"));
    let speakers = report
        .items
        .iter()
        .filter(|item| item.construct == "speaker")
        .collect::<Vec<_>>();
    assert_eq!(speakers.len(), 2, "{speakers:#?}");
    for item in &speakers {
        assert_eq!(item.action, Action::Unresolved);
        assert_eq!(item.diagnostic.code.as_str(), "RECITE_VALIDATE051");
    }
    assert!(speakers[0].diagnostic.help.is_some(), "suggests `operator`");
    assert!(speakers[1].diagnostic.help.is_some(), "says to declare it");
    assert_eq!(
        speakers[0].provenance.location,
        Location::Json {
            pointer: "/0/text".into()
        },
        "points at the first use in the input"
    );
    assert_eq!(
        speakers[1].provenance.location,
        Location::Json {
            pointer: "/2/text".into()
        }
    );
}

#[test]
fn other_native_errors_still_reject_an_import_with_unknown_speakers() {
    let report = speaker_import(
        r#"{"block":"node","text":"text","speaker":"speaker","target":"next"}"#,
        r#"[{"node":"start","text":"Hello.","speaker":"operatr","next":"nowhere"}]"#,
    );
    assert_eq!(report.status, ImportStatus::Invalid);
    assert!(
        report
            .native_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_VALIDATE051")
    );
    assert!(report.items.iter().all(|item| item.construct != "speaker"));
}

#[test]
fn accepted_source_ids_are_preserved_and_other_ids_have_explicit_mappings() {
    let mapping: FieldMapping =
        serde_json::from_str(r#"{"block":"node","text":"text","id":"id"}"#).expect("mapping");
    let source = r#"[{"node":"a","text":"First.","id":"ffee6p8"},{"node":"a","text":"Second.","id":"Legacy Key"}]"#;
    let report = import(ImportRequest {
        family: SourceFamily::Json,
        file: "input.json",
        source,
        mapping: Some(&mapping),
        schema: None,
        default_block: true,
        target_sources: None,
    })
    .expect("report");
    assert_eq!(report.status, ImportStatus::Complete);
    assert!(report.source.contains("> @ffee6p8"));
    let generated = report
        .mappings
        .iter()
        .find(|mapping| mapping.original_id.as_deref() == Some("Legacy Key"))
        .expect("mapping");
    assert!(
        report
            .source
            .contains(&format!("@{}", generated.generated_id))
    );
    assert!(recite_core::is_valid_stable_id(&generated.generated_id));
    assert_eq!(
        generated.generated_id.len(),
        recite_core::GENERATED_STABLE_ID_LEN
    );
}
