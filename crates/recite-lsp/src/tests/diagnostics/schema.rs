use lsp_types::Position;
use serde_json::json;
use tempfile::TempDir;

use super::super::support::{Harness, file_uri, full_change, write_file};

#[path = "schema_close.rs"]
mod schema_close;

pub(super) fn did_open_publishes_schema_backed_semantic_diagnostics() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", semantic_schema());
    let harness = harness_for_schema(&temp);
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));

    harness.did_open(
        source_uri,
        1,
        concat!(
            ":: start default\n",
            "> intro@d0c93b6fa28cacf4b1b0 speaker=rhea talker=ghost sfx=missing portrait=neutral\n",
            "  [ghost]Hello[/ghost]\n",
            "> missing_context@6e9cc3e62c1b68602ec8 portrait=neutral\n",
            "  Missing context.\n",
            "? ask@8d454f8d90909d59c202 requires=(missing_condition(hazel))\n",
            "  Ask?\n",
            "  -> END\n",
            "! immediate missing_effect(snap)\n",
        ),
    );
    let published = harness.recv_publish_diagnostics();

    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        [
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE031",
            "RECITE_VALIDATE022",
            "RECITE_VALIDATE022",
            "RECITE_VALIDATE032",
            "RECITE_VALIDATE034",
            "RECITE_VALIDATE017"
        ]
    );
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 49));
    assert_eq!(published.diagnostics[2].range.start, Position::new(1, 76));
    assert_eq!(published.diagnostics[5].range.start, Position::new(3, 48));

    harness.finish();
}

pub(super) fn did_save_publishes_schema_backed_diagnostics_for_closed_project_files() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", semantic_schema());
    write_file(
        temp.path(),
        "dialogue/saved.recite",
        concat!(":: start default\n", "! immediate missing_effect(snap)\n"),
    );
    let saved_uri = file_uri(&temp.path().join("dialogue/saved.recite"));
    let harness = harness_for_schema(&temp);

    harness.did_save(saved_uri.clone());
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.uri, saved_uri);
    assert_eq!(published.version, None);
    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        ["RECITE_VALIDATE017"]
    );

    harness.finish();
}

pub(super) fn did_save_schema_reloads_and_republishes_source_diagnostics() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", semantic_schema());
    let schema_uri = file_uri(&temp.path().join("schema.json"));
    let harness = harness_for_schema(&temp);
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));

    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> intro@e3ca7d7cd6e07208a608\n",
            "  Hello.\n",
            "! immediate play_sfx(missing)\n",
        ),
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        ["RECITE_VALIDATE021"]
    );

    let updated_schema = semantic_schema().replace(
        "\"sound\": { \"values\": [\"snap\"] }",
        "\"sound\": { \"values\": [\"snap\", \"missing\"] }",
    );
    write_file(temp.path(), "schema.json", &updated_schema);
    harness.did_save(schema_uri.clone());

    let schema_clear = harness.recv_publish_diagnostics();
    assert_eq!(schema_clear.uri, schema_uri);
    assert!(schema_clear.diagnostics.is_empty());

    let source_refresh = harness.recv_publish_diagnostics();
    assert_eq!(source_refresh.uri, source_uri);
    assert!(source_refresh.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn did_save_schema_reloads_from_non_canonical_schema_uri() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", semantic_schema());
    let schema_uri = file_uri(&temp.path().join(".").join("schema.json"));
    let harness = harness_for_schema(&temp);
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));

    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> intro@e3ca7d7cd6e07208a608\n",
            "  Hello.\n",
            "! immediate play_sfx(missing)\n",
        ),
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        ["RECITE_VALIDATE021"]
    );

    let updated_schema = semantic_schema().replace(
        "\"sound\": { \"values\": [\"snap\"] }",
        "\"sound\": { \"values\": [\"snap\", \"missing\"] }",
    );
    write_file(temp.path(), "schema.json", &updated_schema);
    harness.did_save(schema_uri);

    let schema_clear = harness.recv_publish_diagnostics();
    assert!(schema_clear.diagnostics.is_empty());

    let source_refresh = harness.recv_publish_diagnostics();
    assert_eq!(source_refresh.uri, source_uri);
    assert!(source_refresh.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn did_save_keeps_unsaved_schema_overlay() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema = "schema_version = 1\n[producer]\nid = \"dialogue\"\n";
    write_file(temp.path(), "schema.toml", schema);
    let schema_uri = file_uri(&temp.path().join("schema.toml"));
    let harness = harness_for_toml_schema(&temp);

    harness.did_open(schema_uri.clone(), 7, "not a schema\n");
    let overlay = harness.recv_publish_diagnostics();
    assert_eq!(overlay.uri, schema_uri);
    assert_eq!(overlay.version, Some(7));
    assert!(!overlay.diagnostics.is_empty());

    write_file(temp.path(), "schema.toml", schema);
    harness.did_save(schema_uri.clone());
    let after_save = harness.recv_publish_diagnostics();
    assert_eq!(after_save.uri, schema_uri);
    assert_eq!(after_save.version, Some(7));
    assert!(!after_save.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn watched_schema_refresh_keeps_unsaved_schema_overlay() {
    use lsp_types::notification::{DidChangeWatchedFiles, Notification};
    use lsp_types::{DidChangeWatchedFilesParams, FileChangeType, FileEvent};

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema = "schema_version = 1\n[producer]\nid = \"dialogue\"\n";
    write_file(temp.path(), "schema.toml", schema);
    let schema_uri = file_uri(&temp.path().join("schema.toml"));
    let harness = harness_for_toml_schema(&temp);

    harness.did_open(schema_uri.clone(), 9, "not a schema\n");
    let _ = harness.recv_publish_diagnostics();
    write_file(temp.path(), "schema.toml", schema);
    harness.send_notification(
        DidChangeWatchedFiles::METHOD,
        DidChangeWatchedFilesParams {
            changes: vec![FileEvent {
                uri: schema_uri.clone(),
                typ: FileChangeType::CHANGED,
            }],
        },
    );
    let after_watch = harness.recv_publish_diagnostics();
    assert_eq!(after_watch.uri, schema_uri);
    assert_eq!(after_watch.version, Some(9));
    assert!(!after_watch.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn valid_schema_overlay_clears_diagnostics_with_new_version() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema = "schema_version = 1\n[producer]\nid = \"dialogue\"\n";
    write_file(temp.path(), "schema.toml", schema);
    let schema_uri = file_uri(&temp.path().join("schema.toml"));
    let harness = harness_for_toml_schema(&temp);

    harness.did_open(schema_uri.clone(), 4, "not a schema\n");
    let malformed = harness.recv_publish_diagnostics();
    assert_eq!(malformed.uri, schema_uri);
    assert_eq!(malformed.version, Some(4));
    assert!(!malformed.diagnostics.is_empty());

    harness.did_change(schema_uri.clone(), 5, vec![full_change(schema)]);
    let cleared = harness.recv_publish_diagnostics();
    assert_eq!(cleared.uri, schema_uri);
    assert_eq!(cleared.version, Some(5));
    assert!(cleared.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn did_close_schema_alias_clears_exact_uri() {
    schema_close::did_close_schema_alias_clears_exact_uri();
}

pub(super) fn retired_schema_alias_close_clears_and_reopens() {
    schema_close::retired_schema_alias_close_clears_and_reopens();
}

fn diagnostic_codes(diagnostics: &[lsp_types::Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| match diagnostic.code.as_ref() {
            Some(lsp_types::NumberOrString::String(code)) => code.as_str(),
            _ => "<missing>",
        })
        .collect()
}

fn harness_for_schema(temp: &TempDir) -> Harness {
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": {
            "schema": schema_path.display().to_string()
        }
    }))
    .0
}

fn harness_for_toml_schema(temp: &TempDir) -> Harness {
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.toml");
    Harness::start_with_result(json!({
        "capabilities": {},
        "rootUri": root_uri.as_str(),
        "initializationOptions": {
            "schema": schema_path.display().to_string()
        }
    }))
    .0
}

fn semantic_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "registries": {
    "sound": { "values": ["snap"] }
  },
  "speakers": {
    "hazel": {},
    "rhea": {}
  },
  "conditions": {
    "trust_gte": {
      "params": [
        { "name": "actor_a", "type": "speaker" },
        { "name": "actor_b", "type": "speaker" },
        { "name": "threshold", "type": "int" }
      ]
    }
  },
  "effects": {
    "play_sfx": {
      "modes": ["immediate"],
      "params": [{ "name": "sound_effect", "type": "registry:sound" }]
    }
  },
  "metadata_domains": {
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "hazel": ["neutral"],
        "rhea": ["flat"]
      },
      "missing_context": { "policy": "diagnostic" }
    }
  },
  "metadata": {
    "talker": {
      "targets": ["line"],
      "type": "speaker"
    },
    "sfx": {
      "targets": ["line"],
      "type": "registry:sound"
    },
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    }
  },
  "markup": {
    "slow": {
      "requires_closing": true,
      "translatable": true
    }
  }
}"#
}
