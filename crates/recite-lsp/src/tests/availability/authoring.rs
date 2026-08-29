use lsp_types::request::{Completion, HoverRequest, Request as LspRequest};
use lsp_types::{CompletionResponse, HoverContents, Position};
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
        ":: start default speaker=hazel\n",
        "> line@186493315915b423ae7a speaker=hazel portrait=wry\n",
        "> inherited@5c2f4a0a59d5b8f4d9e1 portrait=smile\n",
        "> line-choice-site@1a2b3c4d5e6f708192a3 speaker=hazel portrait=hazel_only\n",
        "? choice@0a1b2c3d4e5f60718293 portrait=hazel_only\n",
        "> malformed-mood@b8ddf7c38a39af4d9be2 mood=\"warm\" stage=fallback_stage\n",
        "> repeated-mood@47c4b31a79f6e8d0c2a1 mood=warm mood=cold stage=fallback_stage\n",
        "> unmapped-mood@6ee55288cd8572cabeba mood=cold stage=fallback_stage\n",
        "> quoted-speaker@0d3a9c5e7f1b2d4a6c8e speaker=\"rhea\" portrait=flat\n",
        "  Hello.\n",
        "  smile in prose.\n",
        "-> saved_block\n",
        ":if can_talk\n",
        "! immediate play_sfx\n",
        "> projection_terms@06a9ba6082698e2a3a77\n",
        "  actor_skill choice_skill_prefix prefix skill_check_prefix item portrait_by_speaker\n",
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

    assert!(metadata.contains("Schema producer adapter 'authoring-fixtures'"));

    let metadata_domain = hover_text(
        harness
            .hover(
                source_uri.clone(),
                position_inside(source, "portrait_by_speaker"),
            )
            .expect("metadata domain hover"),
    );
    assert!(metadata_domain.contains("portraits-v1"));

    let registry = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "item"))
            .expect("registry hover"),
    );
    assert!(registry.contains("items-v1"));

    let value = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "wry"))
            .expect("metadata value hover"),
    );
    assert!(value.contains("Produced by fixture"));

    let inherited_value = hover_text(
        harness
            .hover(source_uri.clone(), position_inside(source, "smile"))
            .expect("inherited metadata value hover"),
    );
    assert!(inherited_value.contains("portrait_by_speaker' (hazel)"));

    let line_value = hover_text(
        harness
            .hover(
                source_uri.clone(),
                position_after(
                    source,
                    "line-choice-site@1a2b3c4d5e6f708192a3 speaker=hazel portrait=haz",
                ),
            )
            .expect("line selector contextual metadata value hover"),
    );
    assert!(line_value.contains("hazel_only"));

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "choice@0a1b2c3d4e5f60718293 portrait=haz",),
            )
            .is_none(),
        "choice selector must not inherit the block speaker context"
    );

    let fallback_value = hover_text(
        harness
            .hover(
                source_uri.clone(),
                position_after(
                    source,
                    "unmapped-mood@6ee55288cd8572cabeba mood=cold stage=",
                ),
            )
            .expect("fallback contextual metadata value hover"),
    );
    assert!(fallback_value.contains("stage_all/fallback_stage"));
    let fallback_completion = completion_labels(
        harness
            .completion(
                source_uri.clone(),
                position_after(
                    source,
                    "unmapped-mood@6ee55288cd8572cabeba mood=cold stage=",
                ),
            )
            .expect("fallback contextual metadata completion"),
    );
    assert_eq!(fallback_completion, ["fallback_stage"]);

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "mood=\"warm\" stage=")
            )
            .is_none(),
        "quoted metadata selector must not use configured fallback"
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "mood=warm mood=cold stage=")
            )
            .is_none(),
        "repeated metadata selectors must not use configured fallback"
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "speaker=\"rhea\" portrait=")
            )
            .is_none(),
        "quoted speaker must not inherit the block default"
    );

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_inside(source, "smile in prose"),
            )
            .is_none(),
        "ordinary prose tokens must not receive schema-value hover"
    );

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

fn completion_labels(response: CompletionResponse) -> Vec<String> {
    match response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    }
}
