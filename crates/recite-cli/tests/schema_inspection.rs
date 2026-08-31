#![cfg(test)]

mod support;

use std::fs;

use serde_json::Value;
use support::{OutputExt, recite, run, stderr, write_file};
use tempfile::TempDir;

const STANDALONE: &str = include_str!("../../../fixtures/schema/valid/standalone.toml");
const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn inspect(path: &std::path::Path) -> std::process::Output {
    run(recite().arg("inspect-schema").arg(path))
}

fn json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("inspection JSON")
}

#[test]
fn standalone_toml_projects_summary_with_origins_and_scoped_fingerprints() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(temp.path(), "schema.toml", STANDALONE);
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    let projection = json(&output);
    assert_eq!(projection["format_version"], 1);
    assert_eq!(projection["source"]["format"], "standalone_toml");
    assert_eq!(projection["source"]["path"]["encoding"], "utf8");
    assert_eq!(
        projection["source"]["path"]["value"],
        schema.to_str().expect("UTF-8 test path")
    );
    assert_eq!(projection["source"]["read_only"], false);
    assert_eq!(projection["ownership"]["kind"], "standalone");
    assert_eq!(projection["producer"]["id"], "standalone-example");
    assert_eq!(projection["freshness"]["status"], "unavailable");
    assert_eq!(
        projection["fingerprints"]["producer_inputs"]["manifest"][0]["id"],
        "standalone-example"
    );
    assert_eq!(projection["types"][0]["name"], "actor_kind");
    assert_eq!(projection["registries"][0]["name"], "item");
    assert_eq!(projection["metadata_domains"][0]["name"], "tone");
    assert_eq!(projection["types"][0]["provenance"]["origin"], Value::Null);
}

#[test]
fn generated_manifest_is_read_only_and_repeatably_projected() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(temp.path(), "schema.json", GENERATED);
    let before = fs::read(&schema).expect("fixture bytes");
    let first = inspect(&schema);
    let second = inspect(&schema);

    first.assert_success().assert_stderr("");
    second.assert_success().assert_stderr("");
    assert_eq!(first.stdout, second.stdout);
    let projection = json(&first);
    assert_eq!(projection["source"]["format"], "generated_json");
    assert_eq!(projection["source"]["read_only"], true);
    assert_eq!(projection["ownership"]["kind"], "generated");
    assert_eq!(projection["producer"]["id"], "example");
    assert_eq!(
        projection["registries"][0]["definition"]["origin"]["extensions"]["engine:resource_kind"],
        "item"
    );
    assert_eq!(
        projection["fingerprints"]["producer_inputs"]["registries"]["item"][0]["id"],
        "content/items"
    );
    assert_eq!(projection["freshness"]["reason"], "no_comparison_snapshot");
    assert_eq!(
        projection["types"][0]["capability"]["actions"],
        serde_json::json!(["read_only_generated"])
    );
    assert_eq!(
        projection["capability"]["actions"],
        serde_json::json!(["read_only_generated"])
    );
    assert_eq!(
        projection["capability"]["producer_actions"],
        serde_json::json!([])
    );
    assert_eq!(
        before,
        fs::read(&schema).expect("fixture remains unchanged")
    );
}

#[test]
fn empty_standalone_source_exposes_only_edit_capability() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "empty.toml",
        "schema_version = 1\n\n[producer]\nid = \"standalone-empty\"\n",
    );
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    let projection = json(&output);
    assert_eq!(projection["types"], serde_json::json!([]));
    assert_eq!(
        projection["capability"],
        serde_json::json!({
            "actions": ["edit_standalone_source"],
            "unavailable_reasons": [],
            "producer_actions": []
        })
    );
}

#[test]
fn empty_generated_manifest_is_read_only_without_actions() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "empty.json",
        r#"{
          "schema_version": 1,
          "producer": {"kind": "adapter", "id": "empty"}
        }"#,
    );
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    assert_eq!(
        json(&output)["capability"],
        serde_json::json!({
            "actions": ["read_only_generated"],
            "unavailable_reasons": [],
            "producer_actions": []
        })
    );
}

#[test]
fn generated_type_and_selector_names_keep_canonical_prefixes() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "prefixed.json",
        r#"{
          "schema_version": 1,
          "types": {"stage": {"kind": "enum", "values": ["ready"]}},
          "conditions": {"stage_is_ready": {"returns": "enum:stage"}},
          "metadata_domains": {
            "emotion_by_subject": {
              "kind": "contextual",
              "selector": "metadata:subject",
              "values_by_context": {"rhea": ["calm"]},
              "missing_context": {"policy": "diagnostic"}
            }
          }
        }"#,
    );
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    let projection = json(&output);
    assert_eq!(
        projection["conditions"][0]["definition"]["returns"],
        "enum:stage"
    );
    assert_eq!(
        projection["metadata_domains"][0]["definition"]["selector"],
        "metadata:subject"
    );
}

#[test]
fn generated_origin_extensions_retain_json_value_types() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "extensions.json",
        r#"{
          "schema_version": 1,
          "registries": {
            "item": {
              "values": ["key"],
              "origin": {
                "kind": "asset_path",
                "id": "items/key.item",
                "engine:priority": 3
              }
            }
          }
        }"#,
    );
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    assert_eq!(
        json(&output)["registries"][0]["definition"]["origin"]["extensions"]["engine:priority"],
        3
    );
}

#[test]
fn generated_manifest_without_producer_metadata_does_not_invent_owner_or_freshness() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "unowned.json",
        r#"{
          "schema_version": 1,
          "speakers": {"rhea": {}}
        }"#,
    );
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    let projection = json(&output);
    assert_eq!(projection["source"]["read_only"], true);
    assert_eq!(projection["ownership"]["kind"], "unavailable");
    assert_eq!(projection["producer"], Value::Null);
    assert_eq!(projection["freshness"]["status"], "unavailable");
    assert_eq!(projection["freshness"]["reason"], "no_producer_metadata");
    assert_eq!(
        projection["capability"],
        serde_json::json!({
            "actions": ["unavailable"],
            "unavailable_reasons": ["unknown_source_owner"],
            "producer_actions": []
        })
    );
    assert_eq!(
        projection["speakers"][0]["capability"],
        serde_json::json!({
            "actions": ["unavailable"],
            "unavailable_reasons": ["unknown_source_owner"],
            "producer_actions": []
        })
    );
}

#[test]
fn supported_malformed_and_unknown_inputs_are_typed_failures() {
    let temp = TempDir::new().expect("tempdir");
    let malformed = write_file(temp.path(), "broken.json", "{ nope");
    let malformed_output = inspect(&malformed);
    malformed_output.assert_failure().assert_exit_code(1);
    assert!(stderr(&malformed_output).contains("RECITE_SCHEMA001"));
    assert!(stderr(&malformed_output).contains("malformed generated_json schema input"));
    assert!(malformed_output.stdout.is_empty());

    let malformed_toml = write_file(temp.path(), "broken.toml", "schema_version = [");
    let malformed_toml_output = inspect(&malformed_toml);
    malformed_toml_output.assert_failure().assert_exit_code(1);
    assert!(stderr(&malformed_toml_output).contains("RECITE_SCHEMA001"));
    assert!(stderr(&malformed_toml_output).contains("malformed standalone_toml schema input"));
    assert!(malformed_toml_output.stdout.is_empty());

    let unknown = write_file(temp.path(), "schema.yaml", "schema_version: 1\n");
    let unknown_output = inspect(&unknown);
    unknown_output.assert_failure().assert_exit_code(1);
    assert!(stderr(&unknown_output).contains("unsupported schema inspection format `yaml`"));
    assert!(unknown_output.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn non_utf8_schema_path_uses_an_explicit_machine_encoding() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = TempDir::new().expect("tempdir");
    let name = OsString::from_vec(b"schema-\xff.toml".to_vec());
    let schema = temp.path().join(name);
    fs::write(&schema, STANDALONE).expect("schema fixture");
    let output = inspect(&schema);

    output.assert_success().assert_stderr("");
    assert_eq!(json(&output)["source"]["path"]["encoding"], "unix_bytes");
}
