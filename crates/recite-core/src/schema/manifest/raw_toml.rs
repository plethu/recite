use super::{
    TomlSpanIndex,
    producer::RawProducerOrigin,
    raw::{
        Named, RawManifest, RawPresentationAffordanceFieldSource,
        RawPresentationProjectorDefinition, RawProjectionInputSource, RawValue,
    },
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Restore TOML float tokens after deserializing into the shared raw model.
///
/// `toml_edit::de` supplies floating values to generic serde visitors as
/// `f64`, which loses spelling and precision.  The parsed TOML CST remains the
/// source of truth for the token; this pass only updates the raw-value fields
/// that can carry schema literals before the canonical lowerer validates them.
pub(crate) fn preserve_toml_float_lexemes(
    raw: &mut RawManifest,
    source: &str,
    spans: &TomlSpanIndex,
) {
    preserve_value(
        &mut raw.schema_version,
        &["schema_version".to_owned()],
        source,
        spans,
    );

    for condition in &mut raw.conditions {
        let prefix = vec![
            "conditions".to_owned(),
            condition.name.clone(),
            "availability_reason".to_owned(),
            "args".to_owned(),
        ];
        let Some(mapping) = condition.value.availability_reason.as_mut() else {
            continue;
        };
        for argument in &mut mapping.args {
            let mut path = prefix.clone();
            path.push(argument.name.clone());
            preserve_value(&mut argument.value, &path, source, spans);
        }
    }

    for registry in &mut raw.registries {
        preserve_origin_fields(
            registry.value.origin.as_mut(),
            &path(&["registries", &registry.name, "origin"]),
            source,
            spans,
        );
        preserve_origin_map(
            registry.value.value_origins.as_mut(),
            &path(&["registries", &registry.name, "value_origins"]),
            source,
            spans,
        );
    }
    for reason in &mut raw.availability_reasons {
        preserve_origin_fields(
            reason.value.origin.as_mut(),
            &path(&["availability_reasons", &reason.name, "origin"]),
            source,
            spans,
        );
    }
    for domain in &mut raw.metadata_domains {
        preserve_origin_fields(
            domain.value.origin.as_mut(),
            &path(&["metadata_domains", &domain.name, "origin"]),
            source,
            spans,
        );
        preserve_origin_map(
            domain.value.value_origins.as_mut(),
            &path(&["metadata_domains", &domain.name, "value_origins"]),
            source,
            spans,
        );
        preserve_origin_map(
            domain.value.context_origins.as_mut(),
            &path(&["metadata_domains", &domain.name, "context_origins"]),
            source,
            spans,
        );
    }

    for projector in &mut raw.presentation_projectors {
        for (index, input) in projector.value.inputs.iter_mut().enumerate() {
            if let RawProjectionInputSource::Literal { value } = &mut input.source {
                let path = vec![
                    "presentation_projectors".to_owned(),
                    projector.name.clone(),
                    "inputs".to_owned(),
                    format!("[{index}]"),
                    "source".to_owned(),
                    "value".to_owned(),
                ];
                preserve_value(value, &path, source, spans);
            }
        }
        preserve_projection_output_lexemes(projector, source, spans);
    }
}

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn preserve_origin_fields(
    origin: Option<&mut RawProducerOrigin>,
    parent: &[String],
    source: &str,
    spans: &TomlSpanIndex,
) {
    let Some(origin) = origin else { return };
    preserve_json_map(&mut origin.extensions, parent, source, spans);
}

fn preserve_origin_map(
    value: Option<&mut Value>,
    parent: &[String],
    source: &str,
    spans: &TomlSpanIndex,
) {
    let Some(value) = value else { return };
    preserve_json_value(value, parent, source, spans);
}

fn preserve_json_map(
    values: &mut BTreeMap<String, Value>,
    parent: &[String],
    source: &str,
    spans: &TomlSpanIndex,
) {
    for (key, value) in values {
        let mut child = parent.to_vec();
        child.push(key.clone());
        preserve_json_value(value, &child, source, spans);
    }
}

fn preserve_json_value(value: &mut Value, path: &[String], source: &str, spans: &TomlSpanIndex) {
    if let Some(range) = spans.float_range(path)
        && let Some(token) = source.get(range)
        && let Ok(number) = serde_json::Number::from_str(&normalize_float_token(token))
    {
        *value = Value::Number(number);
        return;
    }
    match value {
        Value::Array(values) => {
            for (index, value) in values.iter_mut().enumerate() {
                let mut child = path.to_vec();
                child.push(format!("[{index}]"));
                preserve_json_value(value, &child, source, spans);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                let mut child = path.to_vec();
                child.push(key.clone());
                preserve_json_value(value, &child, source, spans);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn preserve_projection_output_lexemes(
    projector: &mut Named<RawPresentationProjectorDefinition>,
    source: &str,
    spans: &TomlSpanIndex,
) {
    for output in &mut projector.value.outputs {
        for field in &mut output.value.fields {
            if let RawPresentationAffordanceFieldSource::Literal { value } = &mut field.value.source
            {
                let path = vec![
                    "presentation_projectors".to_owned(),
                    projector.name.clone(),
                    "outputs".to_owned(),
                    output.name.clone(),
                    "fields".to_owned(),
                    field.name.clone(),
                    "source".to_owned(),
                    "value".to_owned(),
                ];
                preserve_value(value, &path, source, spans);
            }
        }
    }
}

fn preserve_value(value: &mut RawValue, path: &[String], source: &str, spans: &TomlSpanIndex) {
    if let Some(range) = spans.float_range(path)
        && let Some(token) = source.get(range)
    {
        *value = RawValue::Number(normalize_float_token(token));
        return;
    }

    match value {
        RawValue::Array(values) => {
            for (index, value) in values.iter_mut().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(format!("[{index}]"));
                preserve_value(value, &child_path, source, spans);
            }
        }
        RawValue::Object(fields) => {
            for (key, value) in fields {
                let mut child_path = path.to_vec();
                child_path.push(key.clone());
                preserve_value(value, &child_path, source, spans);
            }
        }
        RawValue::Null | RawValue::Bool(_) | RawValue::Number(_) | RawValue::String(_) => {}
    }
}

fn normalize_float_token(token: &str) -> String {
    let syntax = token
        .chars()
        .filter(|character| *character != '_')
        .enumerate()
        .filter_map(|(index, character)| (index != 0 || character != '+').then_some(character))
        .collect::<String>();
    serde_json::Number::from_str(&syntax).map_or(syntax, |number| number.to_string())
}
