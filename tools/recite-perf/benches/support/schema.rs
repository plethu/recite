//! Shared schema loading: all consumers pay this cost, not just Writer.
use recite_core::schema::{ProjectSchema, SchemaLoadReport, SchemaSourceLoadReport};

pub(super) struct Input {
    pub text: String,
    declarations: usize,
}

pub(super) fn json(count: usize) -> Input {
    let conditions: serde_json::Map<_, _> = (0..count)
        .map(|i| {
            (
                format!("gate_{i:06}"),
                serde_json::json!({"params":[],"returns":"bool"}),
            )
        })
        .collect();
    Input {
        declarations: count,
        text: serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1,"conditions":conditions,
        }))
        .unwrap_or_else(|error| panic!("valid manifest: {error}")),
    }
}

pub(super) fn toml(count: usize) -> Input {
    let mut text = "schema_version = 1\n[producer]\nid = \"cost-probe\"\n".to_owned();
    for i in 0..count {
        text.push_str(&format!(
            "[conditions.gate_{i:06}]\nparams = []\nreturns = \"bool\"\n"
        ));
    }
    Input {
        text,
        declarations: count,
    }
}

fn validate(input: &Input, schema: &ProjectSchema) {
    assert_eq!(schema.conditions.len(), input.declarations);
    for (i, (name, condition)) in schema.conditions.iter().enumerate() {
        assert_eq!(name, &format!("gate_{i:06}"));
        assert!(condition.params.is_empty());
        assert_eq!(
            condition.returns,
            recite_core::schema::ConditionReturnType::Bool
        );
    }
}

pub(super) fn validate_json((input, report): (Input, SchemaLoadReport)) {
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    validate(
        &input,
        &report.schema.unwrap_or_else(|| panic!("loaded schema")),
    );
}
pub(super) fn validate_toml((input, report): (Input, SchemaSourceLoadReport)) {
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let source = report
        .source
        .unwrap_or_else(|| panic!("loaded schema source"));
    assert_eq!(source.source_text(), input.text);
    validate(&input, source.schema());
}
