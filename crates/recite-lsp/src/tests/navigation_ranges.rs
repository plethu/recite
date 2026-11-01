use crate::tests::support::{position_after, position_after_in_header, position_inside};
use lsp_types::Position;
use tempfile::TempDir;

use super::support::{Harness, file_uri, write_file};

pub(super) fn typed_clause_and_schema_ranges_exclude_delimiters() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(serde_json::json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/ranges.recite"));
    let source = concat!(
        ":: start default speaker=hazel\r\n",
        "? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint @y0txed0\r\n",
        "  😀 innkeeper_trust_hint, ordinary prose\r\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let clause = harness
        .hover(source_uri.clone(), position_inside(source, "requires"))
        .expect("typed requires clause hover");
    assert_eq!(
        clause.range,
        Some(lsp_types::Range::new(
            position_after_in_header(source, "y0txed0", "? "),
            position_after_in_header(source, "y0txed0", "requires=(trust_gte(hazel, rhea, 3))"),
        )),
        "requires range must stop before the following metadata field",
    );

    let prose = harness
        .hover(source_uri, position_after(source, "  😀 innkeeper_trust"))
        .expect("schema prose hover");
    assert_eq!(
        prose.range,
        Some(lsp_types::Range::new(
            position_after(source, "  😀 "),
            position_after(source, "  😀 innkeeper_trust_hint"),
        )),
        "schema token range must stop before the comma",
    );
    harness.finish();
}

pub(super) fn condition_marker_completion_and_hover_follow_parser_boundaries() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(serde_json::json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/markers.recite"));
    let source = concat!(
        ":: start default\n",
        "\t:if\n",
        "\t:match\n",
        "\t:if\ttrust_\n",
        "\tordinary prose :if trust_gte\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:if"))
                .expect("bare :if completion")
        )
        .contains(&"trust_gte".to_owned())
    );
    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:match"))
                .expect("bare :match completion")
        )
        .contains(&"trust_gte".to_owned())
    );
    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:if\ttrust_"),)
                .expect("tab-separated condition completion")
        )
        .contains(&"trust_gte".to_owned())
    );

    let marker_hover = harness
        .hover(source_uri.clone(), Position::new(3, 2))
        .expect("hover on the exact :if marker");
    assert_eq!(
        marker_hover.range,
        Some(lsp_types::Range::new(
            Position::new(3, 1),
            Position::new(3, 4),
        ))
    );
    assert!(
        harness
            .hover(source_uri.clone(), Position::new(3, 0))
            .is_none(),
        "indentation before a marker must not receive marker hover"
    );
    assert!(
        harness
            .hover(source_uri.clone(), Position::new(3, 4))
            .is_none(),
        "whitespace and arguments after a marker must not receive marker hover"
    );
    assert!(
        harness
            .hover(source_uri, position_after(source, "\tordinary prose "))
            .is_none(),
        "ordinary prose must not receive marker hover"
    );
    harness.finish();
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
