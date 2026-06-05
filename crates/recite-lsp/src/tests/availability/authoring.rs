use lsp_types::request::{Completion, HoverRequest, Request as LspRequest};
use lsp_types::{CompletionResponse, HoverContents, Position};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, uri, write_file};

pub(super) fn initialize_advertises_completion_and_hover() {
    let (harness, result) = Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        }
    }));

    assert!(result.capabilities.completion_provider.is_some());
    assert!(result.capabilities.hover_provider.is_some());

    harness.finish();
}

pub(super) fn completes_requires_conditions_and_parameterless_reasons() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
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
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "? ask requires=(tr\n",
            "? ask_reason requires=(trust_gte(hazel, rhea, 3)) reason=inn\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    let requires = completion_labels(
        harness
            .completion(source_uri.clone(), Position::new(1, 18))
            .expect("requires completion"),
    );
    assert_eq!(requires, ["thread_stage", "trust_gte"]);

    let reasons = completion_labels(
        harness
            .completion(source_uri, Position::new(2, 59))
            .expect("reason completion"),
    );
    assert_eq!(reasons, ["innkeeper_trust_hint"]);

    harness.finish();
}

pub(super) fn completes_project_and_schema_authoring_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", authoring_schema());
    write_file(temp.path(), "saved.recite", ":: saved_block\n");
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
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
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default\n",
        "> line speaker=ha portrait=w\n",
        "  Hello.\n",
        "-> sav\n",
        ":if can_t\n",
        "! immediate play\n",
        "> metadata_line por\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let speakers = completion_labels(
        harness
            .completion(source_uri.clone(), position_after(source, "speaker=ha"))
            .expect("speaker completion"),
    );
    assert_eq!(speakers, ["hazel", "rhea"]);

    let blocks = completion_labels(
        harness
            .completion(source_uri.clone(), position_after(source, "-> sav"))
            .expect("block completion"),
    );
    assert_eq!(blocks, ["saved_block", "start"]);

    let conditions = completion_labels(
        harness
            .completion(source_uri.clone(), position_after(source, ":if can_t"))
            .expect("condition completion"),
    );
    assert_eq!(conditions, ["can_talk"]);

    let effects = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "! immediate play"),
            )
            .expect("effect completion"),
    );
    assert_eq!(effects, ["play_sfx"]);

    let metadata_keys = completion_labels(
        harness
            .completion(source_uri, position_after(source, "> metadata_line por"))
            .expect("metadata key completion"),
    );
    assert_eq!(metadata_keys, ["mood", "portrait", "stage"]);

    harness.finish();
}

pub(super) fn completes_metadata_domain_values_from_schema_context() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", authoring_schema());
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
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
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default speaker=rhea\n",
        "> line speaker=hazel portrait=\n",
        "  Hello.\n",
        "> inherited portrait=\n",
        "  Hi.\n",
        "> by_tone mood=warm stage=\n",
        "  Welcome.\n",
        "> fallback stage=\n",
        "  There.\n",
        "> empty mood=\n",
        "  Quiet.\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let speaker_context = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "speaker=hazel portrait="),
            )
            .expect("speaker contextual metadata completion"),
    );
    assert_eq!(speaker_context, ["smile", "wry"]);

    let inherited_speaker = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "> inherited portrait="),
            )
            .expect("inherited speaker contextual metadata completion"),
    );
    assert_eq!(inherited_speaker, ["flat"]);

    let metadata_key_context = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "mood=warm stage="),
            )
            .expect("metadata-key contextual metadata completion"),
    );
    assert_eq!(metadata_key_context, ["market"]);

    let fallback = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "> fallback stage="),
            )
            .expect("fallback contextual metadata completion"),
    );
    assert_eq!(fallback, ["fallback_stage"]);

    let empty = completion_labels(
        harness
            .completion(source_uri, position_after(source, "> empty mood="))
            .expect("empty contextual metadata completion"),
    );
    assert!(empty.is_empty());

    harness.finish();
}

pub(super) fn hover_distinguishes_unavailable_and_hidden_choices() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/hover.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "? ask requires=(trust_gte(hazel, rhea, 3))\n",
            ":if trust_gte(hazel, rhea, 3)\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    let requires = hover_text(
        harness
            .hover(source_uri.clone(), Position::new(1, 9))
            .expect("requires hover"),
    );
    assert!(requires.contains("keeps the choice visible"));

    let hidden = hover_text(
        harness
            .hover(source_uri, Position::new(2, 1))
            .expect(":if hover"),
    );
    assert!(hidden.contains("structurally omits"));

    harness.finish();
}

pub(super) fn hover_uses_utf16_positions_after_non_ascii_prefix() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/hover-utf16.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "? ask accent=é requires=(trust_gte(hazel, rhea, 3))\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    let hover = harness
        .hover(source_uri, Position::new(1, 16))
        .expect("requires hover after non-ascii prefix");

    assert_eq!(
        hover.range.expect("hover range").start,
        Position::new(1, 15)
    );

    harness.finish();
}

pub(super) fn hover_describes_schema_and_project_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", authoring_schema());
    write_file(temp.path(), "saved.recite", ":: saved_block\n");
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
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
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default\n",
        "> line speaker=hazel portrait=wry\n",
        "  Hello.\n",
        "-> saved_block\n",
        ":if can_talk\n",
        "! immediate play_sfx\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let speaker = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "hazel"))
            .expect("speaker hover"),
    );
    assert!(speaker.contains("Hazel"));

    let metadata = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "portrait"))
            .expect("metadata hover"),
    );
    assert!(metadata.contains("metadata key `portrait`"));

    let block = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "saved_block"))
            .expect("block hover"),
    );
    assert!(block.contains("current project index"));

    let condition = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "can_talk"))
            .expect("condition hover"),
    );
    assert!(condition.contains("condition -> bool"));

    let effect = hover_text(
        harness
            .hover(source_uri, position_inside(source, "play_sfx"))
            .expect("effect hover"),
    );
    assert!(effect.contains("effect request"));

    harness.finish();
}

pub(super) fn malformed_completion_and_hover_params_return_invalid_params() {
    let mut harness = Harness::start();

    let completion = harness.raw_request_response(Completion::METHOD, json!({"bad": true}));
    assert_eq!(
        completion.error.expect("completion error").code,
        lsp_server::ErrorCode::InvalidParams as i32
    );

    let hover = harness.raw_request_response(HoverRequest::METHOD, json!({"bad": true}));
    assert_eq!(
        hover.error.expect("hover error").code,
        lsp_server::ErrorCode::InvalidParams as i32
    );

    harness.finish();
}

fn completion_labels(response: CompletionResponse) -> Vec<String> {
    match response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    }
}

fn hover_text(hover: lsp_types::Hover) -> String {
    match hover.contents {
        HoverContents::Markup(content) => content.value,
        other => panic!("unexpected hover contents: {other:?}"),
    }
}

fn position_after(source: &str, needle: &str) -> Position {
    let index = source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
        + needle.len();
    position_for_byte_index(source, index)
}

fn position_inside(source: &str, needle: &str) -> Position {
    let index = source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
        + 1;
    position_for_byte_index(source, index)
}

fn position_for_byte_index(source: &str, byte_index: usize) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for character_value in source[..byte_index].chars() {
        if character_value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(character_value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}

fn authoring_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "speakers": {
    "hazel": { "display_name": "Hazel" },
    "rhea": {}
  },
  "conditions": {
    "can_talk": { "params": [] }
  },
  "effects": {
    "play_sfx": {
      "modes": ["immediate"],
      "params": []
    }
  },
  "metadata_domains": {
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "hazel": ["smile", "wry"],
        "rhea": ["flat"]
      },
      "missing_context": { "policy": "fallback", "domain": "portrait_all" }
    },
    "portrait_all": {
      "kind": "flat",
      "values": ["flat", "smile", "wry"]
    },
    "stage_by_mood": {
      "kind": "contextual",
      "selector": "metadata:mood",
      "values_by_context": {
        "warm": ["market"]
      },
      "missing_context": { "policy": "fallback", "domain": "stage_all" }
    },
    "stage_all": {
      "kind": "flat",
      "values": ["fallback_stage"]
    },
    "mood_by_tone": {
      "kind": "contextual",
      "selector": "metadata:tone",
      "values_by_context": {
        "bright": ["warm"]
      },
      "missing_context": { "policy": "empty" }
    }
  },
  "metadata": {
    "mood": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "mood_by_tone"
    },
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    },
    "stage": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "stage_by_mood"
    }
  }
}"#
}
