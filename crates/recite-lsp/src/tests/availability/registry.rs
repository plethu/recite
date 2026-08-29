use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

pub(super) fn filters_registry_metadata_completion_to_source_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let mut schema: serde_json::Value =
        serde_json::from_str(authoring_schema()).expect("authoring schema JSON");
    schema["registries"]["mixed"] = json!({
        "values": [
            "valid_symbol",
            "rhea.face",
            "hyphen-name",
            "true",
            "false",
            "$hero",
            "two words",
            "punctuation!",
            "[array]"
        ]
    });
    schema["metadata"]["mixed"] = json!({
        "targets": ["choice"],
        "type": "registry:mixed"
    });
    let schema_text = serde_json::to_string_pretty(&schema).expect("serialise mixed schema");
    write_file(temp.path(), "schema.json", &schema_text);
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default speaker=hazel\n",
        "? mixed@e1f2031425364758697a mixed=\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let labels = completion_labels(
        harness
            .completion(
                source_uri,
                position_after(source, "mixed@e1f2031425364758697a mixed="),
            )
            .expect("mixed registry metadata completion"),
    );
    assert_eq!(labels, ["hyphen-name", "rhea.face", "valid_symbol"]);
    assert!(
        !labels.is_empty(),
        "valid registry symbols must remain available"
    );

    harness.finish();
}

pub(super) fn filters_enum_metadata_completion_to_source_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let mut schema: serde_json::Value =
        serde_json::from_str(authoring_schema()).expect("authoring schema JSON");
    schema["types"]["mixed_kind"] = json!({
        "kind": "enum",
        "values": [
            "valid_symbol",
            "rhea.face",
            "hyphen-name",
            "true",
            "false",
            "$hero",
            "two words",
            "punctuation!",
            "[array]"
        ]
    });
    schema["metadata"]["mixed_kind"] = json!({
        "targets": ["choice"],
        "type": "enum:mixed_kind"
    });
    let schema_text = serde_json::to_string_pretty(&schema).expect("serialise mixed enum schema");
    write_file(temp.path(), "schema.json", &schema_text);
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default speaker=hazel\n",
        "? mixed@e1f2031425364758697a mixed_kind=\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let labels = completion_labels(
        harness
            .completion(
                source_uri,
                position_after(source, "mixed@e1f2031425364758697a mixed_kind="),
            )
            .expect("mixed enum metadata completion"),
    );
    assert_eq!(labels, ["hyphen-name", "rhea.face", "valid_symbol"]);
    assert!(
        !labels.is_empty(),
        "valid enum symbols must remain available"
    );

    harness.finish();
}

pub(super) fn filters_contextual_domain_completion_to_source_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let mut schema: serde_json::Value =
        serde_json::from_str(authoring_schema()).expect("authoring schema JSON");
    schema["metadata_domains"]["mixed_by_speaker"] = json!({
        "kind": "contextual",
        "selector": "field:speaker",
        "values_by_context": {
            "hazel": [
                "valid_symbol",
                "rhea.face",
                "hyphen-name",
                "true",
                "$hero",
                "two words",
                "punctuation!",
                "[array]"
            ]
        },
        "missing_context": { "policy": "empty" }
    });
    schema["metadata"]["mixed_domain"] = json!({
        "targets": ["line"],
        "type": "symbol",
        "domain": "mixed_by_speaker"
    });
    let schema_text = serde_json::to_string_pretty(&schema).expect("serialise mixed domain schema");
    write_file(temp.path(), "schema.json", &schema_text);
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default speaker=hazel\n",
        "> mixed@e1f2031425364758697a mixed_domain=\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let labels = completion_labels(
        harness
            .completion(
                source_uri,
                position_after(source, "mixed@e1f2031425364758697a mixed_domain="),
            )
            .expect("mixed contextual domain completion"),
    );
    assert_eq!(labels, ["hyphen-name", "rhea.face", "valid_symbol"]);
    assert!(
        !labels.is_empty(),
        "valid domain symbols must remain available"
    );

    harness.finish();
}

fn completion_labels(response: lsp_types::CompletionResponse) -> Vec<String> {
    match response {
        lsp_types::CompletionResponse::Array(items) => {
            items.into_iter().map(|item| item.label).collect()
        }
        lsp_types::CompletionResponse::List(list) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
    }
}
