use recite_core::schema::{export_schema_manifest_json, load_schema_manifest_str};
use serde_json::{Value, json};

fn manifest() -> Value {
    let projector = json!({
        "candidates": { "kind": "runtime_event", "event": "dialogue" },
        "queries": {
            "a": { "function": "constant", "args": [] },
            "z": { "function": "pass", "args": [{ "query_result": "a" }] }
        },
        "outputs": {
            "a": { "target": "candidate", "kind": "badge", "slot": "prefix", "fields": {} },
            "z": { "target": "candidate", "kind": "badge", "slot": "suffix", "fields": {} }
        }
    });
    json!({
        "schema_version": 1,
        "projection_queries": {
            "constant": { "returns": "int" },
            "pass": { "params": [{ "name": "value", "type": "int" }], "returns": "int" }
        },
        "presentation_projectors": { "a": projector.clone(), "z": projector }
    })
}

fn reverse_objects(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for child in object.values_mut() {
                reverse_objects(child);
            }
            *object = std::mem::take(object).into_iter().rev().collect();
        }
        Value::Array(values) => {
            for child in values {
                reverse_objects(child);
            }
        }
        _ => {}
    }
}

#[test]
fn projector_output_and_query_ids_have_canonical_order_independent_of_manifest_order() {
    let source = manifest();
    let mut reversed = source.clone();
    reverse_objects(&mut reversed);
    let load = |source: &Value| {
        let report = load_schema_manifest_str("order.json", &source.to_string());
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        report.schema.expect("valid schema")
    };
    let first = load(&source);
    let reversed = load(&reversed);
    assert_eq!(
        first
            .presentation_projectors
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
    for projector in first.presentation_projectors.values() {
        assert_eq!(
            projector
                .queries
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
        assert_eq!(
            projector
                .outputs
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
    }
    assert_eq!(first, reversed);
    assert_eq!(
        first.canonical_fingerprint(),
        reversed.canonical_fingerprint()
    );
    assert_eq!(
        export_schema_manifest_json(&first).expect("export"),
        export_schema_manifest_json(&reversed).expect("export")
    );
}

#[test]
fn query_dependencies_must_precede_their_consumer_in_canonical_id_order() {
    let mut source = manifest();
    source["presentation_projectors"]["a"]["queries"] = json!({
        "z": { "function": "constant", "args": [] },
        "a": { "function": "pass", "args": [{ "query_result": "z" }] }
    });
    for reverse in [false, true] {
        if reverse {
            reverse_objects(&mut source);
        }
        let report = load_schema_manifest_str("forward.json", &source.to_string());
        assert!(
            report.schema.is_none(),
            "source ordering cannot make a forward canonical reference valid"
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown query result 'z'")),
            "{:?}",
            report.diagnostics
        );
    }
}

#[test]
fn canonical_query_validation_retains_source_spans_for_repeated_function_names() {
    let source = r#"{
  "schema_version": 1,
  "projection_queries": { "pass": { "params": [{ "name": "value", "type": "int" }], "returns": "int" } },
  "presentation_projectors": { "p": {
    "candidates": { "kind": "runtime_event", "event": "dialogue" },
    "inputs": [{ "name": "seed", "source": { "kind": "literal", "value": 1 }, "type": "int" }],
    "queries": {
      "z": { "function": "pass", "args": [{ "input": "seed" }] },
      "a": { "function": "pass", "args": [{ "query_result": "z" }] }
    },
    "outputs": {}
  } }
}"#;
    let report = load_schema_manifest_str("query-spans.json", source);
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("unknown query result 'z'"))
        .expect("forward dependency rejected");
    assert_eq!(
        diagnostic.span.start.line(),
        9,
        "diagnostic belongs to canonical query a, which is second in source"
    );
}
