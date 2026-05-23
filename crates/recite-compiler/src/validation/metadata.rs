use std::collections::BTreeSet;

use recite_core::{
    Metadata, MetadataDefinition, MetadataEntry, MetadataTarget, ScalarValue, SchemaTypeDefinition,
    SchemaTypeRef, SourceFile, SourceSpan, Value,
};

use super::project;
use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_metadata_schema(
        &mut self,
        source_file: &'a SourceFile,
        metadata: &'a Metadata,
        target: MetadataTarget,
    ) {
        let Some(schema) = self.schema else {
            return;
        };

        let mut seen_non_repeatable = BTreeSet::new();
        for entry in metadata {
            let key_span = metadata_key_span(source_file, entry);
            let value_span = metadata_value_span(source_file, entry);

            let Some(definition) = schema.metadata.get(entry.key.as_str()) else {
                self.diagnostics.push(diagnostics::unknown_metadata_key(
                    &entry.key,
                    key_span.clone(),
                ));
                continue;
            };

            if !definition.targets.contains(&target) {
                self.diagnostics.push(diagnostics::invalid_metadata_target(
                    &entry.key,
                    display_metadata_target(target),
                    key_span.clone(),
                ));
            }

            if !definition.repeatable && !seen_non_repeatable.insert(entry.key.as_str()) {
                self.diagnostics
                    .push(diagnostics::duplicate_metadata_key(&entry.key, key_span));
            }

            self.validate_metadata_value_schema(entry, definition, value_span);
        }
    }

    fn validate_metadata_value_schema(
        &mut self,
        entry: &MetadataEntry,
        definition: &MetadataDefinition,
        span: SourceSpan,
    ) {
        let Some(schema) = self.schema else {
            return;
        };

        let valid = match &definition.type_ref {
            SchemaTypeRef::String => {
                matches!(entry.value, Value::Scalar(ScalarValue::String(_)))
            }
            SchemaTypeRef::Int => {
                matches!(entry.value, Value::Scalar(ScalarValue::Integer(_)))
            }
            SchemaTypeRef::Float => {
                matches!(entry.value, Value::Scalar(ScalarValue::Float(_)))
            }
            SchemaTypeRef::Bool => {
                matches!(entry.value, Value::Scalar(ScalarValue::Boolean(_)))
            }
            SchemaTypeRef::Speaker => {
                let Some(value) =
                    self.metadata_reference_value(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                if !schema.speakers.contains_key(value) {
                    self.invalid_metadata_value(entry, &definition.type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Enum(enum_name) => {
                let Some(value) =
                    self.metadata_reference_value(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                let Some(schema_type_definition) = schema.types.get(enum_name) else {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                    return;
                };
                let SchemaTypeDefinition::Enum(enum_definition) = schema_type_definition;
                if !enum_definition.values.contains(value) {
                    self.invalid_metadata_value(entry, &definition.type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Registry(registry_name) => {
                let Some(value) =
                    self.metadata_reference_value(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                let Some(registry) = schema.registries.get(registry_name) else {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                    return;
                };
                if !registry.values.contains(value) {
                    self.invalid_metadata_value(entry, &definition.type_ref, value, span);
                }
                return;
            }
        };

        if !valid {
            self.wrong_metadata_value_type(entry, &definition.type_ref, span);
        }
    }

    fn metadata_reference_value<'b>(
        &mut self,
        entry: &'b MetadataEntry,
        type_ref: &SchemaTypeRef,
        span: SourceSpan,
    ) -> Option<&'b str> {
        let value = metadata_scalar_string(entry);
        if value.is_none() {
            self.wrong_metadata_value_type(entry, type_ref, span);
        }
        value
    }

    fn wrong_metadata_value_type(
        &mut self,
        entry: &MetadataEntry,
        expected: &SchemaTypeRef,
        span: SourceSpan,
    ) {
        self.diagnostics
            .push(diagnostics::wrong_metadata_value_type(
                &entry.key,
                expected,
                display_metadata_value_type(&entry.value),
                span,
            ));
    }

    fn invalid_metadata_value(
        &mut self,
        entry: &MetadataEntry,
        expected: &SchemaTypeRef,
        value: &str,
        span: SourceSpan,
    ) {
        self.diagnostics.push(diagnostics::invalid_metadata_value(
            &entry.key, expected, value, span,
        ));
    }
}

fn metadata_key_span(source_file: &SourceFile, entry: &MetadataEntry) -> SourceSpan {
    entry
        .key_span
        .clone()
        .or_else(|| entry.source_span.clone())
        .unwrap_or_else(|| metadata_value_span(source_file, entry))
}

fn metadata_value_span(source_file: &SourceFile, entry: &MetadataEntry) -> SourceSpan {
    entry
        .value_span
        .clone()
        .or_else(|| entry.source_span.clone())
        .unwrap_or_else(|| project::first_source_span(&[source_file]))
}

fn metadata_scalar_string(entry: &MetadataEntry) -> Option<&str> {
    match &entry.value {
        Value::Scalar(ScalarValue::String(value)) => Some(value),
        Value::Scalar(_) | Value::Array(_) => None,
    }
}

fn display_metadata_value_type(value: &Value) -> &'static str {
    match value {
        Value::Scalar(ScalarValue::String(_)) => "string",
        Value::Scalar(ScalarValue::Integer(_)) => "int",
        Value::Scalar(ScalarValue::Float(_)) => "float",
        Value::Scalar(ScalarValue::Boolean(_)) => "bool",
        Value::Array(_) => "array",
    }
}

fn display_metadata_target(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Line => "line",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Project => "project",
    }
}
