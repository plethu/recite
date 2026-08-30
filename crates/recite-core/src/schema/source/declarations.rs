use super::edit::new_declaration;
use super::toml::{SchemaSourceEdit, SchemaSourceEditError};
use crate::EffectMode;
use crate::schema::{
    AvailabilityReasonDefinition, ConditionDefinition, ConditionReturnType, EffectDefinition,
    ParameterDefinition, SchemaTypeRef,
};
use toml_edit::DocumentMut;

pub(super) fn apply_declaration_edit(
    document: &mut DocumentMut,
    edit: SchemaSourceEdit,
) -> Result<(), SchemaSourceEditError> {
    match edit {
        SchemaSourceEdit::AddCondition { name, definition } => {
            add_condition(document, &name, &definition)
        }
        SchemaSourceEdit::AddEffect { name, definition } => {
            add_effect(document, &name, &definition)
        }
        SchemaSourceEdit::AddAvailabilityReason { name, definition } => {
            add_reason(document, &name, &definition)
        }
        _ => Ok(()),
    }
}

fn add_condition(
    document: &mut DocumentMut,
    name: &str,
    definition: &ConditionDefinition,
) -> Result<(), SchemaSourceEditError> {
    let table = new_declaration(document, "conditions", name)?;
    insert_params(table, &definition.params);
    let returns = match &definition.returns {
        ConditionReturnType::Bool => "bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("enum:{name}"),
    };
    table.insert("returns", toml_edit::value(returns));
    if let Some(mapping) = &definition.availability_reason {
        let mut reason = toml_edit::Table::new();
        reason.insert("reason", toml_edit::value(mapping.reason.as_str()));
        let mut args = toml_edit::Table::new();
        for (name, binding) in &mapping.args {
            let mut value = toml_edit::InlineTable::new();
            match binding {
                crate::AvailabilityReasonArgBinding::ConditionParam(param) => {
                    value.insert("kind", toml_edit::Value::from("binding"));
                    value.insert("name", toml_edit::Value::from(param.as_str()));
                }
                crate::AvailabilityReasonArgBinding::Literal(literal) => {
                    value.insert("kind", toml_edit::Value::from("literal"));
                    value.insert("value", literal_value(literal)?);
                }
            }
            args.insert(name, toml_edit::Item::Value(value.into()));
        }
        reason.insert("args", toml_edit::Item::Table(args));
        table.insert("availability_reason", toml_edit::Item::Table(reason));
    }
    Ok(())
}

fn add_effect(
    document: &mut DocumentMut,
    name: &str,
    definition: &EffectDefinition,
) -> Result<(), SchemaSourceEditError> {
    let table = new_declaration(document, "effects", name)?;
    let mut modes = toml_edit::Array::new();
    for mode in &definition.modes {
        modes.push(match mode {
            EffectMode::Deferred => "deferred",
            EffectMode::Immediate => "immediate",
            EffectMode::Blocking => "blocking",
        });
    }
    table.insert("modes", toml_edit::Item::Value(modes.into()));
    insert_params(table, &definition.params);
    Ok(())
}

fn add_reason(
    document: &mut DocumentMut,
    name: &str,
    definition: &AvailabilityReasonDefinition,
) -> Result<(), SchemaSourceEditError> {
    let table = new_declaration(document, "availability_reasons", name)?;
    table.insert("template", toml_edit::value(&definition.template));
    insert_params(table, &definition.params);
    if let Some(origin) = &definition.origin {
        let mut origin_table = toml_edit::Table::new();
        origin_table.insert("kind", toml_edit::value(&origin.kind));
        origin_table.insert("id", toml_edit::value(&origin.id));
        if let Some(label) = &origin.label {
            origin_table.insert("label", toml_edit::value(label));
        }
        for (key, value) in &origin.extensions {
            origin_table.insert(key, toml_edit::Item::Value(producer_value(value)?));
        }
        table.insert("origin", toml_edit::Item::Table(origin_table));
    }
    Ok(())
}

fn insert_params(table: &mut toml_edit::Table, params: &[ParameterDefinition]) {
    let mut values = toml_edit::Array::new();
    for param in params {
        let mut value = toml_edit::InlineTable::new();
        value.insert("name", toml_edit::Value::from(param.name.as_str()));
        value.insert(
            "type",
            toml_edit::Value::from(type_ref_name(&param.type_ref)),
        );
        values.push(value);
    }
    table.insert("params", toml_edit::Item::Value(values.into()));
}

fn type_ref_name(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", type_ref_name(inner)),
    }
}

fn literal_value(
    literal: &crate::SchemaLiteralValue,
) -> Result<toml_edit::Value, SchemaSourceEditError> {
    match literal {
        crate::SchemaLiteralValue::String(value) => Ok(toml_edit::Value::from(value.as_str())),
        crate::SchemaLiteralValue::Int(value) => Ok(toml_edit::Value::from(*value)),
        crate::SchemaLiteralValue::Float(value) => {
            value.parse::<toml_edit::Value>().map_err(|_| {
                SchemaSourceEditError::InvalidArgument(
                    "float literal is not finite TOML".to_owned(),
                )
            })
        }
        crate::SchemaLiteralValue::Bool(value) => Ok(toml_edit::Value::from(*value)),
    }
}

fn producer_value(
    value: &crate::ProducerMetadataValue,
) -> Result<toml_edit::Value, SchemaSourceEditError> {
    match value {
        crate::ProducerMetadataValue::Null => Err(SchemaSourceEditError::InvalidArgument(
            "origin null values cannot be represented in TOML".to_owned(),
        )),
        crate::ProducerMetadataValue::Bool(value) => Ok(toml_edit::Value::from(*value)),
        crate::ProducerMetadataValue::Number(value) => {
            value.parse::<toml_edit::Value>().map_err(|_| {
                SchemaSourceEditError::InvalidArgument("origin number is not valid TOML".to_owned())
            })
        }
        crate::ProducerMetadataValue::String(value) => Ok(toml_edit::Value::from(value.as_str())),
        crate::ProducerMetadataValue::Array(values) => {
            let mut array = toml_edit::Array::new();
            for value in values {
                array.push(producer_value(value)?);
            }
            Ok(array.into())
        }
        crate::ProducerMetadataValue::Object(values) => {
            let mut table = toml_edit::InlineTable::new();
            for (key, value) in values {
                table.insert(key, producer_value(value)?);
            }
            Ok(table.into())
        }
    }
}
