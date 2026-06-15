use lsp_types::request::{Completion, HoverRequest, Request as LspRequest};
use lsp_types::{HoverContents, Position};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, uri, write_file};

use super::support::{authoring_schema, position_after, position_inside};

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
    assert!(result.capabilities.definition_provider.is_some());
    assert!(result.capabilities.references_provider.is_some());
    assert!(result.capabilities.rename_provider.is_some());

    harness.finish();
}

pub(super) fn hover_distinguishes_unavailable_and_hidden_choices() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/hover.recite");
    let source = concat!(
        ":: start default\n",
        "? ask@72caea2ada317fd50c3e requires=(trust_gte(hazel, rhea, 3))\n",
        ":if trust_gte(hazel, rhea, 3)\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let requires = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "requires"))
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
    let source = concat!(
        ":: start default\n",
        "? ask@12a62353a44a4b0f77d4 accent=é requires=(trust_gte(hazel, rhea, 3))\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let hover = harness
        .hover(source_uri, position_inside(source, "requires"))
        .expect("requires hover after non-ascii prefix");

    assert_eq!(
        hover.range.expect("hover range").start,
        position_after(source, "accent=é ")
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
        "> line@186493315915b423ae7a speaker=hazel portrait=wry\n",
        "  Hello.\n",
        "-> saved_block\n",
        ":if can_talk\n",
        "! immediate play_sfx\n",
        "> projection_terms@06a9ba6082698e2a3a77\n",
        "  actor_skill choice_skill_prefix prefix skill_check_prefix\n",
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
            .hover(source_uri.clone(), position_inside(source, "play_sfx"))
            .expect("effect hover"),
    );
    assert!(effect.contains("effect request"));

    let projection_query = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "actor_skill"))
            .expect("projection query hover"),
    );
    assert!(projection_query.contains("projection query `actor_skill`"));

    let projector = hover_text(
        harness
            .hover(
                source_uri.clone(),
                position_inside(source, "choice_skill_prefix"),
            )
            .expect("presentation projector hover"),
    );
    assert!(projector.contains("presentation projector `choice_skill_prefix`"));

    let output = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, " prefix "))
            .expect("presentation output hover"),
    );
    assert!(output.contains("presentation output `prefix`"));

    let label = hover_text(
        harness
            .hover(source_uri, position_inside(source, "skill_check_prefix"))
            .expect("presentation label hover"),
    );
    assert!(label.contains("presentation label `skill_check_prefix`"));

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

fn hover_text(hover: lsp_types::Hover) -> String {
    match hover.contents {
        HoverContents::Markup(content) => content.value,
        other => panic!("unexpected hover contents: {other:?}"),
    }
}
