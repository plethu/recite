use serde_json::Value;

use super::raw::{
    Named, RawManifest, RawPresentationAffordanceFieldSource, RawProjectionInputSource, RawValue,
};

/// Restore JSON number lexemes after deserializing into the shared raw model.
///
/// `serde_json::Value` retains the lexical representation, whereas the
/// format-neutral visitor receives an `f64` for JSON floats. Only fields
/// represented by `RawValue` are refreshed; all validation remains in the
/// canonical lowerer.
pub(crate) fn preserve_json_number_lexemes(raw: &mut RawManifest, json: &Value) {
    let Value::Object(root) = json else {
        return;
    };
    if let Some(value) = root.get("schema_version") {
        raw.schema_version = RawValue::from_json(value);
    }
    preserve_condition_numbers(raw, root.get("conditions"));
    preserve_projection_numbers(raw, root.get("presentation_projectors"));
}

fn preserve_condition_numbers(raw: &mut RawManifest, conditions: Option<&Value>) {
    let Some(Value::Object(conditions)) = conditions else {
        return;
    };
    for condition in &mut raw.conditions {
        let Some(Value::Object(raw_condition)) = conditions.get(&condition.name) else {
            continue;
        };
        let Some(Value::Object(mapping)) = raw_condition.get("availability_reason") else {
            continue;
        };
        let Some(Value::Object(args)) = mapping.get("args") else {
            continue;
        };
        let Some(raw_mapping) = condition.value.availability_reason.as_mut() else {
            continue;
        };
        for argument in &mut raw_mapping.args {
            if let Some(value) = args.get(&argument.name) {
                argument.value = RawValue::from_json(value);
            }
        }
    }
}

fn preserve_projection_numbers(raw: &mut RawManifest, projectors: Option<&Value>) {
    let Some(Value::Object(projectors)) = projectors else {
        return;
    };
    for projector in &mut raw.presentation_projectors {
        let Some(Value::Object(raw_projector)) = projectors.get(&projector.name) else {
            continue;
        };
        if let Some(Value::Array(inputs)) = raw_projector.get("inputs") {
            for (raw_input, json_input) in projector.value.inputs.iter_mut().zip(inputs) {
                let Value::Object(json_input) = json_input else {
                    continue;
                };
                if let (RawProjectionInputSource::Literal { value }, Some(json_source)) =
                    (&mut raw_input.source, json_input.get("source"))
                    && let Value::Object(json_source) = json_source
                    && let Some(json_value) = json_source.get("value")
                {
                    *value = RawValue::from_json(json_value);
                }
            }
        }
        preserve_projection_output_numbers(projector, raw_projector.get("outputs"));
    }
}

fn preserve_projection_output_numbers(
    projector: &mut Named<super::raw::RawPresentationProjectorDefinition>,
    outputs: Option<&Value>,
) {
    let Some(Value::Object(outputs)) = outputs else {
        return;
    };
    for output in &mut projector.value.outputs {
        let Some(Value::Object(raw_output)) = outputs.get(&output.name) else {
            continue;
        };
        let Some(Value::Object(json_fields)) = raw_output.get("fields") else {
            continue;
        };
        for field in &mut output.value.fields {
            let Some(Value::Object(json_field)) = json_fields.get(&field.name) else {
                continue;
            };
            if let (RawPresentationAffordanceFieldSource::Literal { value }, Some(json_source)) =
                (&mut field.value.source, json_field.get("source"))
                && let Value::Object(json_source) = json_source
                && let Some(json_value) = json_source.get("value")
            {
                *value = RawValue::from_json(json_value);
            }
        }
    }
}
