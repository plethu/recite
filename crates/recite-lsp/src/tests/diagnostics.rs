use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};
use serde_json::json;
use tempfile::TempDir;

use super::support::{Harness, file_uri, full_change, uri, write_file};

pub(super) fn did_open_publishes_source_diagnostics_with_stable_shape() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/broken.recite");

    harness.did_open(uri.clone(), 7, "oops\n:ifx\n:: tavern\n");
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.uri, uri);
    assert_eq!(published.version, Some(7));
    assert_eq!(published.diagnostics.len(), 4);
    let diagnostic = &published.diagnostics[0];
    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String("RECITE_PARSE001".to_owned()))
    );
    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(diagnostic.source.as_deref(), Some("recite"));
    assert_eq!(
        diagnostic.range,
        Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: 0,
                character: 0
            },
        }
    );
    assert_eq!(
        published
            .diagnostics
            .iter()
            .map(|diagnostic| (
                diagnostic.range.start.line,
                diagnostic.range.start.character
            ))
            .collect::<Vec<_>>(),
        [(0, 0), (0, 0), (1, 0), (1, 0)]
    );

    harness.finish();
}

pub(super) fn did_open_publishes_lowering_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/lowering.recite");
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_road\n",
        "  Ask about the road.\n",
        "    Wrong choice indent.\n",
        ":if knows_secret(player)\n",
        "  ! immediate play_sfx(ok)\n",
        "    ! immediate wrong_if_indent()\n",
        ":match thread_stage(thread)\n",
        "    :case ready\n",
        "      ! immediate play_sfx(ok)\n",
        "  :case tired\n",
        ":match mood(player)\n",
        "  :case calm\n",
        "    ! immediate play_sfx(ok)\n",
        "      ! immediate wrong_case_indent()\n",
    );

    harness.did_open(uri, 1, source);
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.diagnostics.len(), 4);
    assert_eq!(
        published
            .diagnostics
            .iter()
            .map(|diagnostic| match diagnostic.code.as_ref() {
                Some(NumberOrString::String(code)) => code.as_str(),
                _ => "<missing>",
            })
            .collect::<Vec<_>>(),
        [
            "RECITE_PARSE007",
            "RECITE_PARSE007",
            "RECITE_PARSE007",
            "RECITE_PARSE007"
        ]
    );

    harness.finish();
}

pub(super) fn did_open_publishes_schema_less_semantic_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/semantic.recite");

    harness.did_open(
        uri.clone(),
        1,
        concat!(
            ":: start default\n",
            ">\n",
            "  Missing line id.\n",
            "> repeated\n",
            "  First repeated line id.\n",
            "> prompt\n",
            "  Prompt.\n",
            "  ? repeated_choice\n",
            "    First repeated choice.\n",
            "    -> missing_block\n",
            "  ? repeated_choice\n",
            "    Second repeated choice.\n",
            "    -> END\n",
            "> repeated\n",
            "  Second repeated line id.\n",
        ),
    );
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.uri, uri.clone());
    assert_eq!(published.version, Some(1));
    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        [
            "RECITE_ID001",
            "RECITE_VALIDATE007",
            "RECITE_ID004",
            "RECITE_ID003"
        ]
    );
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 0));
    assert_eq!(published.diagnostics[1].range.start, Position::new(9, 4));

    harness.did_change(
        uri,
        2,
        vec![full_change(concat!(
            ":: start default\n",
            "> fixed\n",
            "  Fixed.\n",
        ))],
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.version, Some(2));
    assert!(published.diagnostics.is_empty());

    harness.finish();
}

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
            "> intro speaker=rhea talker=ghost sfx=missing portrait=neutral\n",
            "  [ghost]Hello[/ghost]\n",
            "> missing_context portrait=neutral\n",
            "  Missing context.\n",
            "? ask requires=(missing_condition(hazel))\n",
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
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 28));
    assert_eq!(published.diagnostics[2].range.start, Position::new(1, 55));
    assert_eq!(published.diagnostics[5].range.start, Position::new(3, 27));

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
            "> intro\n",
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

pub(super) fn did_close_removes_state_and_clears_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/close.recite");

    harness.did_open(uri.clone(), 1, "oops\n:: tavern\n");
    assert!(!harness.recv_publish_diagnostics().diagnostics.is_empty());
    harness.did_close(uri.clone());
    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.uri, uri);
    assert_eq!(published.version, None);
    assert!(published.diagnostics.is_empty());

    harness.finish();
}

fn diagnostic_codes(diagnostics: &[lsp_types::Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| match diagnostic.code.as_ref() {
            Some(NumberOrString::String(code)) => code.as_str(),
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
