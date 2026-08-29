use lsp_types::{CompletionResponse, HoverContents, NumberOrString, Range};
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
        "> invalid@0a1b2c3d4e5f60718293 speaker=rhea portrait=hazel\n",
        "  Same-name invalid metadata value.\n",
        "> dotted@1a2b3c4d5e6f708192a3 speaker=rhea portrait=rhea.face\n",
        "  Dotted metadata value.\n",
        "> typed@5a6b7c8d9e0f123b4c5d speaker=rhea talker=hazel\n",
        "  Speaker-typed metadata value.\n",
        "> target_comma@6a7b8c9d0e1f234c5d6e speaker=rhea portrait=flat,\n",
        "  Trailing comma is not a metadata value.\n",
        "> target_paren@7a8b9c0d1e2f345d6e7f speaker=rhea portrait=flat)\n",
        "  Trailing parenthesis is not a metadata value.\n",
        "> target_bracket@8a9b0c1d2e3f456e7f80 speaker=rhea portrait=flat]\n",
        "  Trailing bracket is not a metadata value.\n",
        "> comma@2a3b4c5d6e7f8091a2b3 mood=warm, stage=hazel\n",
        "  Trailing comma is not a selector value.\n",
        "> paren@3a4b5c6d7e8f901a2b3c mood=warm) stage=hazel\n",
        "  Trailing parenthesis is not a selector value.\n",
        "> bracket@4a5b6c7d8e9f012a3b4c mood=warm] stage=hazel\n",
        "  Trailing bracket is not a selector value.\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let diagnostics = harness.recv_publish_diagnostics();
    assert_eq!(
        diagnostics
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == Some(NumberOrString::String("RECITE_PARSE008".to_owned()))
            })
            .count(),
        6,
        "each compiler-invalid metadata punctuation must remain a diagnostic",
    );

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

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "speaker=rhea portrait=hazel"),
            )
            .is_none(),
        "an invalid contextual value must not fall through to the hazel speaker",
    );

    let typed_hover = harness
        .hover(
            source_uri.clone(),
            position_after(
                source,
                "typed@5a6b7c8d9e0f123b4c5d speaker=rhea talker=hazel",
            ),
        )
        .expect("speaker-typed metadata value hover");
    assert!(hover_text(typed_hover).contains("Recite speaker `hazel`"));

    let dotted_hover = harness
        .hover(
            source_uri.clone(),
            position_after(source, "speaker=rhea portrait=rhea.face"),
        )
        .expect("dotted contextual metadata value hover");
    assert!(hover_text(dotted_hover.clone()).contains("'rhea.face'"));
    assert_eq!(
        dotted_hover.range,
        Some(Range::new(
            position_after(source, "dotted@1a2b3c4d5e6f708192a3 speaker=rhea portrait="),
            position_after(
                source,
                "dotted@1a2b3c4d5e6f708192a3 speaker=rhea portrait=rhea.face",
            ),
        )),
        "hover range must cover the complete dotted symbol",
    );

    for target in [
        "target_comma@6a7b8c9d0e1f234c5d6e speaker=rhea portrait=",
        "target_paren@7a8b9c0d1e2f345d6e7f speaker=rhea portrait=",
        "target_bracket@8a9b0c1d2e3f456e7f80 speaker=rhea portrait=",
    ] {
        assert!(
            harness
                .hover(
                    source_uri.clone(),
                    position_after(source, &format!("{target}fl")),
                )
                .is_none(),
            "metadata value with trailing punctuation must not resolve: {target:?}",
        );
        assert_eq!(
            completion_labels(
                harness
                    .completion(source_uri.clone(), position_after(source, target))
                    .expect("metadata value completion response"),
            ),
            ["flat", "rhea.face"],
            "completion remains available at the editable value prefix: {target:?}",
        );
    }

    for selector in [
        "mood=warm, stage=",
        "mood=warm) stage=",
        "mood=warm] stage=",
    ] {
        assert!(
            completion_labels(
                harness
                    .completion(source_uri.clone(), position_after(source, selector))
                    .expect("invalid selector completion response"),
            )
            .is_empty(),
            "compiler-invalid selector {selector:?} must not resolve context completion",
        );
        let value = selector.replace("stage=", "stage=hazel");
        assert!(
            harness
                .hover(source_uri.clone(), position_after(source, &value))
                .is_none(),
            "compiler-invalid selector {selector:?} must not resolve contextual hover",
        );
    }

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
