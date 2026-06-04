use lsp_types::{NumberOrString, Position};
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, full_change, uri, write_file};

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
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
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

pub(super) fn schema_diagnostics_validate_live_project_before_filtering_to_uri() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    write_file(
        temp.path(),
        "dialogue/target.recite",
        concat!(
            ":: target default\n",
            "> target_001\n",
            "  Target text.\n",
            "-> END\n",
        ),
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
            ":: start\n",
            "? ask requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "  Ask?\n",
            "  -> dialogue/target.recite::target\n",
        ),
    );

    let published = harness.recv_publish_diagnostics();

    assert!(
        published.diagnostics.is_empty(),
        "open document should validate against saved project files: {:?}",
        published.diagnostics
    );

    harness.finish();
}

pub(super) fn schema_diagnostics_republish_open_references_after_target_changes() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    write_file(
        temp.path(),
        "dialogue/target.recite",
        concat!(
            ":: target default\n",
            "> target_001\n",
            "  Target.\n",
            "-> END\n",
        ),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let start_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let target_uri = file_uri(&temp.path().join("dialogue/target.recite"));
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

    harness.did_open(
        start_uri.clone(),
        1,
        concat!(
            ":: start\n",
            "> start_001\n",
            "  Start.\n",
            "-> dialogue/target.recite::target\n",
        ),
    );
    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

    harness.did_open(
        target_uri.clone(),
        1,
        concat!(
            ":: target default\n",
            "> target_001\n",
            "  Target.\n",
            "-> END\n",
        ),
    );
    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());
    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

    harness.did_change(
        target_uri,
        2,
        vec![full_change(concat!(
            ":: other default\n",
            "> other_001\n",
            "  Other.\n",
            "-> END\n",
        ))],
    );

    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());
    let referenced = harness.recv_publish_diagnostics();

    assert_eq!(referenced.uri, start_uri);
    assert_eq!(
        referenced.diagnostics[0].code,
        Some(NumberOrString::String("RECITE_VALIDATE007".to_owned()))
    );

    harness.finish();
}
