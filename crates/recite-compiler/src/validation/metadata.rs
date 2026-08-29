use std::collections::BTreeSet;

use recite_core::{
    MetadataDefinition, MetadataTarget, SchemaTypeDefinition, SchemaTypeRef, SourceFile,
    SourceMetadata, SourceMetadataEntry, SourceMetadataScalar, SourceMetadataValue, SourceSpan,
};

use super::project;
use super::state::Validator;
use crate::diagnostics;

pub(super) struct MetadataValidationContext<'a> {
    pub(super) target: MetadataTarget,
    pub(super) line_speaker: Option<&'a str>,
    pub(super) block_default_speaker: Option<&'a str>,
    pub(super) metadata: &'a SourceMetadata,
}

impl<'a> Validator<'a> {
    pub(super) fn validate_metadata_schema(
        &mut self,
        source_file: &'a SourceFile,
        context: MetadataValidationContext<'a>,
    ) {
        let Some(schema) = self.schema else {
            return;
        };

        let mut seen_non_repeatable = BTreeSet::new();
        for entry in context.metadata {
            let key_span = metadata_key_span(source_file, entry);
            let value_span = metadata_value_span(source_file, entry);

            let Some(definition) = schema.metadata.get(entry.key.as_str()) else {
                self.diagnostics.push(diagnostics::unknown_metadata_key(
                    &entry.key,
                    key_span.clone(),
                ));
                continue;
            };

            if !definition.targets.contains(&context.target) {
                self.diagnostics.push(diagnostics::invalid_metadata_target(
                    &entry.key,
                    context.target,
                    key_span.clone(),
                ));
            }

            if !definition.repeatable && !seen_non_repeatable.insert(entry.key.as_str()) {
                self.diagnostics
                    .push(diagnostics::duplicate_metadata_key(&entry.key, key_span));
            }

            self.validate_metadata_value_schema(entry, definition, value_span, &context);
        }
    }

    fn validate_metadata_value_schema(
        &mut self,
        entry: &SourceMetadataEntry,
        definition: &MetadataDefinition,
        span: SourceSpan,
        context: &MetadataValidationContext<'a>,
    ) {
        let Some(schema) = self.schema else {
            return;
        };

        match &definition.type_ref {
            SchemaTypeRef::String => {
                if !metadata_value_scalars_match(&entry.value, is_string_literal) {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                }
            }
            SchemaTypeRef::Symbol => {
                let Some(values) =
                    self.metadata_symbol_values(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                if let Some(domain) = &definition.domain {
                    self.validate_metadata_domain_values(entry, domain, &values, span, context);
                }
            }
            SchemaTypeRef::Int => {
                if !metadata_value_scalars_match(&entry.value, is_integer) {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                }
            }
            SchemaTypeRef::Float => {
                if !metadata_value_scalars_match(&entry.value, is_float) {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                }
            }
            SchemaTypeRef::Bool => {
                if !metadata_value_scalars_match(&entry.value, is_bool) {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                }
            }
            SchemaTypeRef::Speaker => {
                let Some(values) =
                    self.metadata_symbol_values(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                for value in values {
                    if !schema.speakers.contains_key(value) {
                        self.invalid_metadata_value(
                            entry,
                            &definition.type_ref,
                            value,
                            span.clone(),
                        );
                    }
                }
            }
            SchemaTypeRef::Enum(enum_name) => {
                let Some(values) =
                    self.metadata_symbol_values(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                let Some(schema_type_definition) = schema.types.get(enum_name) else {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                    return;
                };
                let SchemaTypeDefinition::Enum(enum_definition) = schema_type_definition;
                for value in values {
                    if !enum_definition.values.contains(value) {
                        self.invalid_metadata_value(
                            entry,
                            &definition.type_ref,
                            value,
                            span.clone(),
                        );
                    }
                }
            }
            SchemaTypeRef::Registry(registry_name) => {
                let Some(values) =
                    self.metadata_symbol_values(entry, &definition.type_ref, span.clone())
                else {
                    return;
                };
                let Some(registry) = schema.registries.get(registry_name) else {
                    self.wrong_metadata_value_type(entry, &definition.type_ref, span);
                    return;
                };
                for value in values {
                    if !registry.values.contains(value) {
                        self.invalid_metadata_value(
                            entry,
                            &definition.type_ref,
                            value,
                            span.clone(),
                        );
                    }
                }
            }
            SchemaTypeRef::Array(_) => {
                self.wrong_metadata_value_type(entry, &definition.type_ref, span);
            }
        }
    }

    fn metadata_symbol_values<'b>(
        &mut self,
        entry: &'b SourceMetadataEntry,
        type_ref: &SchemaTypeRef,
        span: SourceSpan,
    ) -> Option<Vec<&'b str>> {
        let values = metadata_symbol_values(entry);
        if values.is_none() {
            self.wrong_metadata_value_type(entry, type_ref, span);
        }
        values
    }

    fn wrong_metadata_value_type(
        &mut self,
        entry: &SourceMetadataEntry,
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
        entry: &SourceMetadataEntry,
        expected: &SchemaTypeRef,
        value: &str,
        span: SourceSpan,
    ) {
        self.diagnostics.push(diagnostics::invalid_metadata_value(
            &entry.key, expected, value, span,
        ));
    }
}

fn metadata_key_span(source_file: &SourceFile, entry: &SourceMetadataEntry) -> SourceSpan {
    entry
        .key_span
        .clone()
        .or_else(|| entry.source_span.clone())
        .unwrap_or_else(|| metadata_value_span(source_file, entry))
}

fn metadata_value_span(source_file: &SourceFile, entry: &SourceMetadataEntry) -> SourceSpan {
    entry
        .value_span
        .clone()
        .or_else(|| entry.source_span.clone())
        .unwrap_or_else(|| project::first_source_span(&[source_file]))
}

fn metadata_symbol_values(entry: &SourceMetadataEntry) -> Option<Vec<&str>> {
    match &entry.value {
        SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol(value)) => Some(vec![value]),
        SourceMetadataValue::Array(values) => values
            .iter()
            .map(|value| match value {
                SourceMetadataScalar::Symbol(value) => Some(value.as_str()),
                SourceMetadataScalar::StringLiteral(_)
                | SourceMetadataScalar::Integer(_)
                | SourceMetadataScalar::Float(_)
                | SourceMetadataScalar::Bool(_) => None,
            })
            .collect(),
        SourceMetadataValue::Scalar(_) => None,
    }
}

fn metadata_value_scalars_match(
    value: &SourceMetadataValue,
    predicate: fn(&SourceMetadataScalar) -> bool,
) -> bool {
    match value {
        SourceMetadataValue::Scalar(value) => predicate(value),
        SourceMetadataValue::Array(values) => values.iter().all(predicate),
    }
}

fn is_string_literal(value: &SourceMetadataScalar) -> bool {
    matches!(value, SourceMetadataScalar::StringLiteral(_))
}

fn is_integer(value: &SourceMetadataScalar) -> bool {
    matches!(value, SourceMetadataScalar::Integer(_))
}

fn is_float(value: &SourceMetadataScalar) -> bool {
    matches!(value, SourceMetadataScalar::Float(_))
}

fn is_bool(value: &SourceMetadataScalar) -> bool {
    matches!(value, SourceMetadataScalar::Bool(_))
}

fn display_metadata_value_type(value: &SourceMetadataValue) -> &'static str {
    match value {
        SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol(_)) => "symbol",
        SourceMetadataValue::Scalar(SourceMetadataScalar::StringLiteral(_)) => "string",
        SourceMetadataValue::Scalar(SourceMetadataScalar::Integer(_)) => "int",
        SourceMetadataValue::Scalar(SourceMetadataScalar::Float(_)) => "float",
        SourceMetadataValue::Scalar(SourceMetadataScalar::Bool(_)) => "bool",
        SourceMetadataValue::Array(_) => "array",
    }
}
