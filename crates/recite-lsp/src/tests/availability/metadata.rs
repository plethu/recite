use lsp_types::{CompletionResponse, HoverContents};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

pub(super) fn hover_prioritizes_contextual_metadata_values() {
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
        ":: start default speaker=hazel\n",
        "> collision@9f4a1b2c3d4e5f607182 speaker=hazel portrait=hazel\n",
        "  Same-name metadata value.\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let hover = hover_text(
        harness
            .hover(
                source_uri.clone(),
                position_after(
                    source,
                    "collision@9f4a1b2c3d4e5f607182 speaker=hazel portrait=",
                ),
            )
            .expect("same-name contextual metadata value hover"),
    );
    assert!(hover.contains("Metadata domain value 'hazel'"));
    assert!(hover.contains("portrait_by_speaker' (hazel)"));

    let completion = completion_labels(
        harness
            .completion(
                source_uri,
                position_after(
                    source,
                    "collision@9f4a1b2c3d4e5f607182 speaker=hazel portrait=",
                ),
            )
            .expect("same-name contextual metadata value completion"),
    );
    assert_eq!(completion, ["hazel", "hazel_only", "smile", "wry"]);

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
