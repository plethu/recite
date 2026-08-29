use super::{
    TomlSpanIndex,
    raw::{
        Named, RawManifest, RawPresentationAffordanceFieldSource,
        RawPresentationProjectorDefinition, RawProjectionInputSource, RawValue,
    },
};
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
