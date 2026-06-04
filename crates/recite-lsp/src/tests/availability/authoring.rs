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
