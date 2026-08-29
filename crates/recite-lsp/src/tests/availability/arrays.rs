use lsp_types::{HoverContents, Range};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

pub(super) fn resolves_metadata_array_elements_by_declared_type() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let mut schema: serde_json::Value =
        serde_json::from_str(authoring_schema()).expect("authoring schema JSON");
    schema["types"]["mood_kind"] = json!({
        "kind": "enum",
        "values": ["calm", "angry"]
    });
    schema["metadata"]["item"] = json!({
        "targets": ["line"],
        "type": "registry:item"
    });
    schema["metadata"]["mood_kind"] = json!({
        "targets": ["line"],
        "type": "enum:mood_kind"
    });
    let schema_text = serde_json::to_string_pretty(&schema).expect("serialise array schema");
    write_file(temp.path(), "schema.json", &schema_text);
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(json!({
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let source = concat!(
        ":: start default speaker=hazel\n",
        "> speakers@e1f2031425364758697a talker=[hazel,rhea]\n",
        "> invalid@f102031425364758697a talker=[hazel,portrait_by_speaker]\n",
        "> trailing@0123456789abcdef0123 talker=[hazel,]\n",
        "> closed@123456789abcdef01234 talker=[hazel,rhea)\n",
        "> registry@23456789abcdef012345 item=[map]\n",
        "> enum@3456789abcdef01234567 mood_kind=[calm,angry]\n",
        "> complete@456789abcdef012345678 talker=[ha\n",
        "> registry_complete@56789abcdef0123456789 item=[m\n",
        "> enum_complete@6789abcdef0123456789a mood_kind=[a\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let first = harness
        .hover(
            source_uri.clone(),
            position_after(source, "speakers@e1f2031425364758697a talker=["),
        )
        .expect("first speaker array element hover");
    assert!(hover_text(first).contains("Recite speaker `hazel`"));
    let second = harness
        .hover(
            source_uri.clone(),
            position_after(source, "speakers@e1f2031425364758697a talker=[hazel,"),
        )
        .expect("second speaker array element hover");
    assert!(hover_text(second).contains("Recite speaker `rhea`"));

    let first_range = harness
        .hover(
            source_uri.clone(),
            position_after(source, "speakers@e1f2031425364758697a talker=["),
        )
        .and_then(|hover| hover.range)
        .expect("first speaker array element range");
    assert_eq!(
        first_range,
        Range::new(
            position_after(source, "speakers@e1f2031425364758697a talker=["),
            position_after(source, "speakers@e1f2031425364758697a talker=[hazel"),
        )
    );

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "invalid@f102031425364758697a talker=[hazel,"),
            )
            .is_none(),
        "an invalid array element must not fall through to a same-name schema symbol",
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "trailing@0123456789abcdef0123 talker=["),
            )
            .is_none(),
        "a trailing-comma array must fail closed",
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after(source, "closed@123456789abcdef01234 talker=[hazel,"),
            )
            .is_none(),
        "an array with an invalid closing delimiter must fail closed",
    );

    assert!(
        hover_text(
            harness
                .hover(
                    source_uri.clone(),
                    position_after(source, "registry@23456789abcdef012345 item=["),
                )
                .expect("registry array element hover"),
        )
        .contains("map")
    );
    assert!(
        hover_text(
            harness
                .hover(
                    source_uri.clone(),
                    position_after(source, "enum@3456789abcdef01234567 mood_kind=["),
                )
                .expect("enum array element hover"),
        )
        .contains("calm")
    );

    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri.clone(),
                    position_after(source, "complete@456789abcdef012345678 talker=[ha"),
                )
                .expect("speaker array completion"),
        ),
        ["hazel", "rhea"],
    );
    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri.clone(),
                    position_after(source, "registry_complete@56789abcdef0123456789 item=[m"),
                )
                .expect("registry array completion"),
        ),
        ["map"],
    );
    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri,
                    position_after(source, "enum_complete@6789abcdef0123456789a mood_kind=[a"),
                )
                .expect("enum array completion"),
        ),
        ["angry", "calm"],
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
