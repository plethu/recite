use lsp_types::CompletionResponse;
use serde_json::json;
use tempfile::TempDir;

use crate::tests::support::{Harness, file_uri, write_file};

use super::support::{authoring_schema, position_after};

pub(super) fn completes_projection_schema_authoring_symbols() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", authoring_schema());
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let schema_uri = file_uri(&schema_path);
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
    let source = concat!(
        "{\n",
        "  \"function\": \"a\",\n",
        "  \"presentation_projectors\": {\n",
        "    \"choice\": {},\n",
        "    \"choice_skill_prefix\": {\n",
        "      \"input\": \"s\",\n",
        "      \"query_result\": \"c\",\n",
        "      \"outputs\": {\n",
        "        \"pre\": {}\n",
        "      },\n",
        "      \"template_id\": \"skill\"\n",
        "    }\n",
        "  }\n",
        "}\n",
    );
    harness.did_open(schema_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let functions = completion_labels(
        harness
            .completion(
                schema_uri.clone(),
                position_after(source, "\"function\": \"a"),
            )
            .expect("projection query function completion"),
    );
    assert_eq!(functions, ["actor_skill"]);

    let inputs = completion_labels(
        harness
            .completion(schema_uri.clone(), position_after(source, "\"input\": \"s"))
            .expect("projection input completion"),
    );
    assert_eq!(inputs, ["skill", "threshold"]);

    let query_results = completion_labels(
        harness
            .completion(
                schema_uri.clone(),
                position_after(source, "\"query_result\": \"c"),
            )
            .expect("projection query result completion"),
    );
    assert_eq!(query_results, ["current"]);

    let projectors = completion_labels(
        harness
            .completion(schema_uri.clone(), position_after(source, "\"choice"))
            .expect("presentation projector completion"),
    );
    assert_eq!(projectors, ["choice_skill_prefix"]);

    let outputs = completion_labels(
        harness
            .completion(schema_uri.clone(), position_after(source, "        \"pre"))
            .expect("presentation output completion"),
    );
    assert_eq!(outputs, ["prefix"]);

    let labels = completion_labels(
        harness
            .completion(
                schema_uri,
                position_after(source, "\"template_id\": \"skill"),
            )
            .expect("presentation label completion"),
    );
    assert_eq!(labels, ["skill_check_prefix"]);

    harness.finish();
}

pub(super) fn scopes_projection_schema_authoring_symbols_to_current_projector() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", projection_scope_schema());
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let schema_uri = file_uri(&schema_path);
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
    let source = concat!(
        "{\n",
        "  \"presentation_projectors\": {\n",
        "    \"choice_skill_prefix\": {\n",
        "      \"input\": \"s\",\n",
        "      \"query_result\": \"c\",\n",
        "      \"outputs\": {\n",
        "        \"p\": {}\n",
        "      },\n",
        "      \"template_id\": \"skill\"\n",
        "    },\n",
        "    \"choice_focus_suffix\": {\n",
        "      \"input\": \"f\"\n",
        "    }\n",
        "  }\n",
        "}\n",
    );
    harness.did_open(schema_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let inputs = completion_labels(
        harness
            .completion(schema_uri.clone(), position_after(source, "\"input\": \"s"))
            .expect("scoped projection input completion"),
    );
    assert_eq!(inputs, ["skill", "threshold"]);

    let query_results = completion_labels(
        harness
            .completion(
                schema_uri.clone(),
                position_after(source, "\"query_result\": \"c"),
            )
            .expect("scoped projection query result completion"),
    );
    assert_eq!(query_results, ["current"]);

    let outputs = completion_labels(
        harness
            .completion(schema_uri.clone(), position_after(source, "        \"p"))
            .expect("scoped projection output completion"),
    );
    assert_eq!(outputs, ["prefix"]);

    let labels = completion_labels(
        harness
            .completion(
                schema_uri,
                position_after(source, "\"template_id\": \"skill"),
            )
            .expect("scoped projection label completion"),
    );
    assert_eq!(labels, ["skill_check_prefix"]);

    harness.finish();
}

pub(super) fn does_not_complete_projection_outputs_in_sibling_objects() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", projection_scope_schema());
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let schema_uri = file_uri(&schema_path);
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
    let source = concat!(
        "{\n",
        "  \"presentation_projectors\": {\n",
        "    \"choice_skill_prefix\": {\n",
        "      \"outputs\": {\n",
        "        \"prefix\": {}\n",
        "      },\n",
        "      \"queries\": {\n",
        "        \"z\": {}\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    );
    harness.did_open(schema_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    assert!(
        harness
            .completion(schema_uri, position_after(source, "        \"z"))
            .is_none(),
        "query keys must not receive projection output completions"
    );

    harness.finish();
}

pub(super) fn does_not_complete_projection_projectors_in_sibling_objects() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", projection_scope_schema());
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let schema_uri = file_uri(&schema_path);
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
    let source = concat!(
        "{\n",
        "  \"presentation_projectors\": {\n",
        "    \"choice_skill_prefix\": {}\n",
        "  },\n",
        "  \"metadata\": {\n",
        "    \"z\": {}\n",
        "  }\n",
        "}\n",
    );
    harness.did_open(schema_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    assert!(
        harness
            .completion(schema_uri, position_after(source, "    \"z"))
            .is_none(),
        "sibling top-level object keys must not receive projection projector completions"
    );

    harness.finish();
}

fn completion_labels(response: CompletionResponse) -> Vec<String> {
    match response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    }
}

fn projection_scope_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "projection_queries": {
    "actor_focus": {
      "params": [{ "name": "focus", "type": "string" }],
      "returns": "int"
    },
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int"
    }
  },
  "presentation_projectors": {
    "choice_focus_suffix": {
      "candidates": { "kind": "runtime_event", "event": "prompt" },
      "inputs": [
        { "name": "focus", "source": { "kind": "literal", "value": "attention" }, "type": "string" }
      ],
      "queries": {
        "focus_level": { "function": "actor_focus", "args": [{ "input": "focus" }] }
      },
      "outputs": {
        "suffix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "suffix",
          "label": {
            "template_id": "focus_suffix",
            "source_text": "{focus}",
            "args": {
              "focus": { "source": { "input": "focus" }, "type": "string" }
            }
          }
        }
      }
    },
    "choice_skill_prefix": {
      "candidates": { "kind": "runtime_event", "event": "prompt" },
      "inputs": [
        { "name": "skill", "source": { "kind": "literal", "value": "speech" }, "type": "string" },
        { "name": "threshold", "source": { "kind": "literal", "value": 3 }, "type": "int" }
      ],
      "queries": {
        "current": { "function": "actor_skill", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "{skill} {current}/{threshold}",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" },
              "current": { "source": { "query_result": "current" }, "type": "int" },
              "threshold": { "source": { "input": "threshold" }, "type": "int" }
            }
          }
        }
      }
    }
  }
}"#
}
