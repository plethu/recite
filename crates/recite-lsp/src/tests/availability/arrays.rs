use lsp_types::{HoverContents, Range};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after_in_header};

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
        "> talker=[hazel,rhea] @szcdxz6\n",
        "> talker=[hazel, rhea] @spaced\n",
        "> item=[map, map] @spaced_registry\n",
        "> mood_kind=[calm, angry] @spaced_enum\n",
        "> talker=[hazel,portrait_by_speaker] @wf8nesa\n",
        "> talker=[hazel,] @ez1fh09\n",
        "> talker=[hazel,rhea) @closed\n",
        "> item=[map] @registry\n",
        "> mood_kind=[calm,angry] @enum\n",
        "> talker=[ha @complete\n",
        "> item=[m @registry_complete\n",
        "> mood_kind=[a @enum_complete\n",
        "> talker=[hazel, r @second_complete\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let first = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "szcdxz6", "talker=["),
        )
        .expect("first speaker array element hover");
    assert!(hover_text(first).contains("Recite speaker `hazel`"));
    let second = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "szcdxz6", "talker=[hazel,"),
        )
        .expect("second speaker array element hover");
    assert!(hover_text(second).contains("Recite speaker `rhea`"));

    let first_range = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "szcdxz6", "talker=["),
        )
        .and_then(|hover| hover.range)
        .expect("first speaker array element range");
    assert_eq!(
        first_range,
        Range::new(
            position_after_in_header(source, "szcdxz6", "talker=["),
            position_after_in_header(source, "szcdxz6", "talker=[hazel"),
        )
    );
    let second_hover = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "spaced", "talker=[hazel, "),
        )
        .expect("spaced second speaker array element hover");
    assert!(hover_text(second_hover).contains("Recite speaker `rhea`"));
    let second_range = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "spaced", "talker=[hazel, "),
        )
        .and_then(|hover| hover.range)
        .expect("spaced second speaker array element range");
    assert_eq!(
        second_range,
        Range::new(
            position_after_in_header(source, "spaced", "talker=[hazel, "),
            position_after_in_header(source, "spaced", "talker=[hazel, rhea"),
        )
    );

    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after_in_header(source, "wf8nesa", "talker=[hazel,"),
            )
            .is_none(),
        "an invalid array element must not fall through to a same-name schema symbol",
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after_in_header(source, "ez1fh09", "talker=["),
            )
            .is_none(),
        "a trailing-comma array must fail closed",
    );
    assert!(
        harness
            .hover(
                source_uri.clone(),
                position_after_in_header(source, "closed", "talker=[hazel,"),
            )
            .is_none(),
        "an array with an invalid closing delimiter must fail closed",
    );

    assert!(
        hover_text(
            harness
                .hover(
                    source_uri.clone(),
                    position_after_in_header(source, "spaced_registry", "item=[map, "),
                )
                .expect("registry array element hover"),
        )
        .contains("map")
    );
    let registry_range = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "spaced_registry", "item=[map, "),
        )
        .and_then(|hover| hover.range)
        .expect("registry array element range");
    assert_eq!(
        registry_range,
        Range::new(
            position_after_in_header(source, "spaced_registry", "item=[map, "),
            position_after_in_header(source, "spaced_registry", "item=[map, map"),
        )
    );
    assert!(
        hover_text(
            harness
                .hover(
                    source_uri.clone(),
                    position_after_in_header(source, "spaced_enum", "mood_kind=[calm, "),
                )
                .expect("enum array element hover"),
        )
        .contains("angry")
    );
    let enum_range = harness
        .hover(
            source_uri.clone(),
            position_after_in_header(source, "spaced_enum", "mood_kind=[calm, "),
        )
        .and_then(|hover| hover.range)
        .expect("enum array element range");
    assert_eq!(
        enum_range,
        Range::new(
            position_after_in_header(source, "spaced_enum", "mood_kind=[calm, "),
            position_after_in_header(source, "spaced_enum", "mood_kind=[calm, angry"),
        )
    );

    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri.clone(),
                    position_after_in_header(source, "complete", "talker=[ha"),
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
                    position_after_in_header(source, "registry_complete", "item=[m"),
                )
                .expect("registry array completion"),
        ),
        ["map"],
    );
    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri.clone(),
                    position_after_in_header(source, "enum_complete", "mood_kind=[a"),
                )
                .expect("enum array completion"),
        ),
        ["angry", "calm"],
    );
    assert_eq!(
        completion_labels(
            harness
                .completion(
                    source_uri,
                    position_after_in_header(source, "second_complete", "talker=[hazel, r"),
                )
                .expect("spaced second speaker array completion"),
        ),
        ["hazel", "rhea"],
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
