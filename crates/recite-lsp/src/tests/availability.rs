use lsp_types::{CompletionResponse, HoverContents, NumberOrString, Position};
use serde_json::json;
use tempfile::TempDir;

use super::support::{Harness, file_uri, uri, write_file};

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

pub(super) fn publishes_choice_availability_parser_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/availability-syntax.recite");
    harness.did_open(
        uri,
        1,
        concat!(
            ":: start default\n",
            "? bad_requires requires=(trust_gte(\n",
            "  Bad requires?\n",
            "? bad_reason reason=trust_too_low(\n",
            "  Bad reason?\n",
            "? old_if if trust_gte(hazel, rhea, 3)\n",
            "  Old if?\n",
        ),
    );

    let published = harness.recv_publish_diagnostics();
    let codes = published
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        codes,
        [
            Some(NumberOrString::String("RECITE_PARSE013".to_owned())),
            Some(NumberOrString::String("RECITE_PARSE008".to_owned())),
            Some(NumberOrString::String("RECITE_PARSE018".to_owned())),
        ]
    );
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 35));
    assert_eq!(published.diagnostics[1].range.start, Position::new(3, 20));
    assert_eq!(published.diagnostics[2].range.start, Position::new(5, 9));

    harness.finish();
}

pub(super) fn publishes_choice_availability_schema_diagnostics() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let harness = Harness::start_with_result(json!({
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
        source_uri,
        1,
        concat!(
            ":: start default\n",
            "? unknown requires=(missing_condition(hazel))\n",
            "  Unknown?\n",
            "  -> END\n",
            "? non_bool requires=(thread_stage(hazel_intro))\n",
            "  Non bool?\n",
            "  -> END\n",
            "? unknown_reason requires=(trust_gte(hazel, rhea, 3)) reason=missing_reason\n",
            "  Unknown reason?\n",
            "  -> END\n",
            "? parameterized_reason requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low\n",
            "  Parameterized reason?\n",
            "  -> END\n",
        ),
    );

    let published = harness.recv_publish_diagnostics();
    let codes = published
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        codes,
        [
            Some(NumberOrString::String("RECITE_VALIDATE034".to_owned())),
            Some(NumberOrString::String("RECITE_VALIDATE038".to_owned())),
            Some(NumberOrString::String("RECITE_VALIDATE039".to_owned())),
            Some(NumberOrString::String("RECITE_VALIDATE040".to_owned())),
        ]
    );
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 20));
    assert_eq!(published.diagnostics[1].range.start, Position::new(4, 21));
    assert_eq!(published.diagnostics[2].range.start, Position::new(7, 61));
    assert_eq!(published.diagnostics[3].range.start, Position::new(10, 67));

    harness.finish();
}

pub(super) fn completes_requires_conditions_and_parameterless_reasons() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
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
