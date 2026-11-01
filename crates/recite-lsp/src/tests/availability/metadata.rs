use lsp_types::{CompletionResponse, HoverContents, NumberOrString, Range};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after, position_after_in_header, position_inside};

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
        "> speaker=hazel portrait=hazel @collision\n",
        "  Same-name metadata value.\n",
        "> speaker=rhea portrait=hazel @atcf38h\n",
        "  Same-name invalid metadata value.\n",
        "> speaker=rhea portrait=rhea.face @qmv6hgp\n",
        "  Dotted metadata value.\n",
        "> speaker=rhea talker=hazel @typed\n",
        "  Speaker-typed metadata value.\n",
        "> speaker=rhea portrait=flat, @target_comma\n",
        "  Trailing comma is not a metadata value.\n",
        "> speaker=rhea portrait=flat) @target_paren\n",
        "  Trailing parenthesis is not a metadata value.\n",
        "> speaker=rhea portrait=flat] @target_bracket\n",
        "  Trailing bracket is not a metadata value.\n",
        "> mood=warm, stage=hazel @comma\n",
        "  Trailing comma is not a selector value.\n",
        "> mood=warm) stage=hazel @paren\n",
        "  Trailing parenthesis is not a selector value.\n",
        "> mood=warm] stage=hazel @bracket\n",
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
                position_after_in_header(source, "collision", "speaker=hazel portrait="),
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
            position_after_in_header(source, "typed", "speaker=rhea talker=hazel"),
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
            position_after_in_header(source, "qmv6hgp", "speaker=rhea portrait="),
            position_after_in_header(source, "qmv6hgp", "speaker=rhea portrait=rhea.face"),
        )),
        "hover range must cover the complete dotted symbol",
    );

    for target in ["target_comma", "target_paren", "target_bracket"] {
        assert!(
            harness
                .hover(
                    source_uri.clone(),
                    position_after_in_header(source, target, "speaker=rhea portrait=fl"),
                )
                .is_none(),
            "metadata value with trailing punctuation must not resolve: {target:?}",
        );
        assert_eq!(
            completion_labels(
                harness
                    .completion(
                        source_uri.clone(),
                        position_after_in_header(source, target, "speaker=rhea portrait="),
                    )
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
                position_after_in_header(source, "collision", "speaker=hazel portrait="),
            )
            .expect("same-name contextual metadata value completion"),
    );
    assert_eq!(completion, ["hazel", "hazel_only", "smile", "wry"]);

    harness.finish();
}

pub(super) fn hover_preserves_choice_reason_clause_resolution() {
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
        ":: start default speaker=hazel\n",
        "? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint @y0txed0\n",
        "? requires=(trust_gte(hazel, rhea, 3)) reason=hazel @fm637hb\n",
        "? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint, @punctuation_reason\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let reason_hover = harness
        .hover(
            source_uri.clone(),
            position_after(source, "reason=innkeeper_trust_hint"),
        )
        .expect("choice reason hover");
    assert!(hover_text(reason_hover).contains("Availability reason 'innkeeper_trust_hint'"));

    let interior_reason_hover = harness
        .hover(
            source_uri.clone(),
            position_inside(source, "innkeeper_trust_hint"),
        )
        .expect("interior choice reason hover");
    assert_eq!(
        interior_reason_hover.range,
        Some(Range::new(
            position_after(source, "reason="),
            position_after(source, "reason=innkeeper_trust_hint"),
        )),
        "reason hover must use the parsed value span",
    );

    assert!(
        harness
            .hover(source_uri.clone(), position_after(source, "reason=hazel"))
            .is_none(),
        "an unknown choice reason must not fall through to the same-named speaker",
    );

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after_in_header(
                    source,
                    "punctuation_reason",
                    "requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust"
                ),
            )
            .is_none(),
        "a compiler-invalid choice reason must not resolve as a complete reason ID",
    );

    assert_eq!(
        completion_labels(
            harness
                .completion(source_uri, position_after(source, "reason=inn"))
                .expect("choice reason completion"),
        ),
        ["innkeeper_trust_hint"],
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
