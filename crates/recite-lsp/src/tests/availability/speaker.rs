use lsp_types::HoverContents;
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

pub(super) fn hover_resolves_choice_speaker_metadata_before_builtin_speakers() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let mut schema: serde_json::Value =
        serde_json::from_str(authoring_schema()).expect("authoring schema JSON");
    schema["metadata_domains"]["choice_speaker_by_mood"] = json!({
        "kind": "contextual",
        "selector": "metadata:mood",
        "values_by_context": {
            "warm": ["hazel"]
        },
        "missing_context": {
            "policy": "empty"
        },
        "context_origins": {
            "warm": {
                "kind": "fixture",
                "id": "choice-speakers/warm"
            }
        },
        "value_origins": {
            "warm": {
                "hazel": {
                    "kind": "fixture",
                    "id": "choice-speakers/warm/hazel"
                }
            }
        },
        "producer_fingerprints": [
            {
                "id": "choice-speakers",
                "kind": "fixture",
                "algorithm": "blake3",
                "value": "choice-speakers-v1"
            }
        ]
    });
    schema["metadata"]["speaker"] = json!({
        "targets": ["choice"],
        "type": "symbol",
        "domain": "choice_speaker_by_mood"
    });
    let schema_text = serde_json::to_string_pretty(&schema).expect("serialise choice schema");
    write_file(temp.path(), "schema.json", &schema_text);
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
        "> ordinary@d1e2f30415263748596a speaker=hazel\n",
        "  Ordinary speaker field.\n",
        "? contextual@e1f2031425364758697a mood=warm speaker=hazel\n",
        "? invalid@f102031425364758697a mood=warm speaker=rhea\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let ordinary_hover = harness
        .hover(
            source_uri.clone(),
            position_after(source, "ordinary@d1e2f30415263748596a speaker=hazel"),
        )
        .expect("ordinary speaker hover");
    assert!(hover_text(ordinary_hover).contains("Recite speaker `hazel`"));

    let contextual_hover = harness
        .hover(
            source_uri.clone(),
            position_after(
                source,
                "contextual@e1f2031425364758697a mood=warm speaker=hazel",
            ),
        )
        .expect("choice metadata speaker hover");
    let contextual_text = hover_text(contextual_hover);
    assert!(contextual_text.contains("Metadata domain value 'hazel'"));
    assert!(contextual_text.contains("choice_speaker_by_mood' (warm)"));
    assert!(contextual_text.contains("Produced by fixture `choice-speakers/warm/hazel`"));

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "mood=warm speaker=rhea")
            )
            .is_none(),
        "an invalid choice metadata speaker must not fall through to the rhea speaker",
    );

    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri,
                    position_after(source, "contextual@e1f2031425364758697a mood=warm speaker="),
                )
                .expect("choice metadata speaker completion"),
        ),
        ["hazel"],
    );

    harness.finish();
}

fn hover_text(hover: lsp_types::Hover) -> String {
    match hover.contents {
        HoverContents::Markup(content) => content.value,
        other => panic!("unexpected hover contents: {other:?}"),
    }
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
