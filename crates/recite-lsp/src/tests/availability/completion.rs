use lsp_types::CompletionResponse;
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

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
    let source = concat!(
        ":: start default\n",
        "? ask@3ec8745874ed3cde29ce requires=(tr\n",
        "? ask_reason@116831dfd01a44eaedb1 requires=(trust_gte(hazel, rhea, 3)) reason=inn\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let requires = completion_labels(
        harness
            .completion(source_uri.clone(), position_after(source, "requires=(tr"))
            .expect("requires completion"),
    );
    assert_eq!(requires, ["thread_stage", "trust_gte"]);

    let reasons = completion_labels(
        harness
            .completion(source_uri, position_after(source, "reason=inn"))
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
        "> line@ff34745f262d15d5b4ce speaker=ha portrait=w\n",
        "  Hello.\n",
        "-> sav\n",
        ":if can_t\n",
        "! immediate play\n",
        "> metadata_line@32a122c362c13a9fba4e por\n",
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
            .completion(
                source_uri,
                position_after(source, "> metadata_line@32a122c362c13a9fba4e por"),
            )
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
        "> line@6932a28590a3e2589912 speaker=hazel portrait=\n",
        "  Hello.\n",
        "> inherited@b5e6d41190a085e8e199 portrait=\n",
        "  Hi.\n",
        "> by_tone@00986d119571fe502b6c mood=warm stage=\n",
        "  Welcome.\n",
        "> dotted@50114af655afb00259aa mood=warm.tone stage=\n",
        "  Dotted.\n",
        "> unmapped@4ca3cc23f3daf60b72ff mood=cold stage=\n",
        "  Cold.\n",
        "> fallback@6ee55288cd8572cabeba stage=\n",
        "  There.\n",
        "> empty@3f999d94c15264aff741 mood=\n",
        "  Quiet.\n",
        "> repeated@cca0e492ff706239ae60 mood=warm mood=bright stage=\n",
        "  Repeated.\n",
        "> malformed@bd10c23e35161e49f8bd mood=\"warm\" stage=\n",
        "  Malformed.\n",
        ":: block speaker=hazel portrait=\n",
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
                position_after(source, "> inherited@b5e6d41190a085e8e199 portrait="),
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

    let dotted_metadata_context = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "mood=warm.tone stage="),
            )
            .expect("dotted metadata-key contextual metadata completion"),
    );
    assert_eq!(dotted_metadata_context, ["market"]);

    let unmapped_context = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "mood=cold stage="),
            )
            .expect("unmapped contextual metadata completion"),
    );
    assert_eq!(unmapped_context, ["fallback_stage"]);

    let fallback = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "> fallback@6ee55288cd8572cabeba stage="),
            )
            .expect("fallback contextual metadata completion"),
    );
    assert_eq!(fallback, ["fallback_stage"]);

    let empty = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "> empty@3f999d94c15264aff741 mood="),
            )
            .expect("empty contextual metadata completion"),
    );
    assert!(empty.is_empty());

    let repeated_selector = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "mood=warm mood=bright stage="),
            )
            .expect("repeated selector metadata completion"),
    );
    assert!(repeated_selector.is_empty());

    let malformed_selector = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "mood=\"warm\" stage="),
            )
            .expect("malformed selector metadata completion"),
    );
    assert!(malformed_selector.is_empty());

    let block_speaker_context = completion_labels(
        harness
            .completion(
                source_uri,
                position_after(source, ":: block speaker=hazel portrait="),
            )
            .expect("block metadata speaker context completion"),
    );
    assert_eq!(block_speaker_context, ["flat", "smile", "wry"]);

    harness.finish();
}

pub(super) fn ignores_non_metadata_authoring_positions() {
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
        ":: start default\n",
        "> li@cc29a2bf4d2fcbf3ce96\n",
        "  prose por\n",
        "  \"function\": \"act\n",
        "? ch@d5b763017bd6c66a9a53\n",
        ":else por\n",
        ":: start def\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    assert!(
        harness
            .completion(source_uri.clone(), position_after(source, "> li"))
            .is_none()
    );
    assert!(
        harness
            .completion(source_uri.clone(), position_after(source, "prose por"))
            .is_none()
    );
    assert!(
        harness
            .completion(
                source_uri.clone(),
                position_after(source, "\"function\": \"act")
            )
            .is_none()
    );
    assert!(
        harness
            .completion(source_uri.clone(), position_after(source, "? ch"))
            .is_none()
    );
    assert!(
        harness
            .completion(source_uri.clone(), position_after(source, ":else por"))
            .is_none()
    );
    assert!(
        harness
            .completion(source_uri, position_after(source, ":: start def"))
            .is_none()
    );

    harness.finish();
}

fn completion_labels(response: CompletionResponse) -> Vec<String> {
    match response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    }
}
