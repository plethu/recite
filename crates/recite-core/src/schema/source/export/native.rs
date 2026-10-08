//! Checks only native values that the manifest transport could otherwise change.
use std::str::FromStr;

use crate::schema::{
    AvailabilityReasonArgBinding, MetadataDomainDefinition, PresentationAffordanceFieldSource,
    ProducerMetadataValue, ProducerOrigin, ProjectSchema, SchemaLiteralValue,
    SchemaProjectionInputSource, is_namespaced_extension_key, schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

use super::super::diagnostics::MALFORMED_SHAPE;

pub(crate) enum NativeExportError {
    Float(String),
    Number(String),
    Extension { owner: String, key: String },
    Fingerprint,
    SemanticDrift,
}

impl NativeExportError {
    pub(crate) fn diagnostic(self) -> Diagnostic {
        let span = SourceSpan::point("<native schema>", crate::source_location::point_one());
        match self {
            Self::Float(owner) => schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-float-not-representable",
                format!("{owner} must be a JSON number finite and representable as f64"),
                span,
                [("owner", DiagnosticArgumentValue::String(owner))],
            ),
            Self::Extension { owner, key } => schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-origin-extension",
                format!("{owner} origin extension '{key}' must be namespaced"),
                span,
                [
                    ("owner", DiagnosticArgumentValue::String(owner)),
                    ("key", DiagnosticArgumentValue::String(key)),
                ],
            ),
            Self::Number(owner) => {
                let detail =
                    format!("{owner} contains a producer number that cannot be preserved as JSON");
                schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-native-invalid",
                    detail.clone(),
                    span,
                    [("detail", DiagnosticArgumentValue::String(detail))],
                )
            }
            Self::Fingerprint => {
                let detail =
                    "non-BLAKE3 producer content fingerprints must contain UTF-8 digest text";
                schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-native-invalid",
                    detail,
                    span,
                    [("detail", DiagnosticArgumentValue::String(detail.to_owned()))],
                )
            }
            Self::SemanticDrift => {
                let detail = "native schema changes typed semantic content when encoded as a manifest; check literal types and numeric spelling";
                schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-native-invalid",
                    detail,
                    span,
                    [("detail", DiagnosticArgumentValue::String(detail.to_owned()))],
                )
            }
        }
    }
}

pub(crate) fn validate_native_export(schema: &ProjectSchema) -> Result<(), NativeExportError> {
    if let Some(fingerprint) = schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.content_fingerprint.as_ref())
        && fingerprint.algorithm().as_str() != "blake3"
        && std::str::from_utf8(fingerprint.digest().as_bytes()).is_err()
    {
        return Err(NativeExportError::Fingerprint);
    }
    validate_literals(schema)?;
    validate_provenance(schema)
}

fn validate_literals(schema: &ProjectSchema) -> Result<(), NativeExportError> {
    for (name, condition) in &schema.conditions {
        if let Some(mapping) = &condition.availability_reason {
            for (argument, binding) in &mapping.args {
                if let AvailabilityReasonArgBinding::Literal(value) = binding {
                    validate_literal(value, &format!("condition '{name}' argument '{argument}'"))?;
                }
            }
        }
    }
    for (name, projector) in &schema.presentation_projectors {
        for input in &projector.inputs {
            if let SchemaProjectionInputSource::Literal(value) = &input.source {
                validate_literal(value, &format!("projector '{name}' input '{}'", input.name))?;
            }
        }
        for (output_name, output) in &projector.outputs {
            for (field_name, field) in &output.fields {
                if let PresentationAffordanceFieldSource::Literal(value) = &field.source {
                    validate_literal(
                        value,
                        &format!("projector '{name}' output '{output_name}' field '{field_name}'"),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn validate_provenance(schema: &ProjectSchema) -> Result<(), NativeExportError> {
    for (name, registry) in &schema.registries {
        let owner = format!("registry '{name}'");
        validate_origins(
            &owner,
            registry
                .origin
                .iter()
                .chain(registry.value_origins.values()),
        )?;
    }
    for (name, reason) in &schema.availability_reasons {
        validate_origins(
            &format!("availability reason '{name}'"),
            reason.origin.iter(),
        )?;
    }
    for (name, domain) in &schema.metadata_domains {
        let owner = format!("metadata domain '{name}'");
        match domain {
            MetadataDomainDefinition::Flat(domain) => validate_origins(
                &owner,
                domain
                    .provenance
                    .origin
                    .iter()
                    .chain(domain.provenance.value_origins.values()),
            )?,
            MetadataDomainDefinition::Contextual(domain) => validate_origins(
                &owner,
                domain
                    .provenance
                    .origin
                    .iter()
                    .chain(domain.provenance.context_origins.values())
                    .chain(
                        domain
                            .provenance
                            .value_origins
                            .values()
                            .flat_map(|origins| origins.values()),
                    ),
            )?,
        }
    }
    Ok(())
}

fn validate_literal(value: &SchemaLiteralValue, owner: &str) -> Result<(), NativeExportError> {
    if let SchemaLiteralValue::Float(value) = value
        && (!value.parse::<f64>().is_ok_and(f64::is_finite)
            || serde_json::Number::from_str(value).is_err())
    {
        return Err(NativeExportError::Float(owner.to_owned()));
    }
    Ok(())
}

fn validate_origins<'a>(
    owner: &str,
    origins: impl IntoIterator<Item = &'a ProducerOrigin>,
) -> Result<(), NativeExportError> {
    for origin in origins {
        for (key, value) in &origin.extensions {
            if !is_namespaced_extension_key(key) {
                return Err(NativeExportError::Extension {
                    owner: owner.to_owned(),
                    key: key.clone(),
                });
            }
            let mut pending = vec![value];
            while let Some(value) = pending.pop() {
                match value {
                    ProducerMetadataValue::Number(number) => {
                        if !serde_json::Number::from_str(number)
                            .is_ok_and(|parsed| parsed.to_string() == *number)
                        {
                            return Err(NativeExportError::Number(format!(
                                "{owner} extension '{key}'"
                            )));
                        }
                    }
                    ProducerMetadataValue::Array(values) => pending.extend(values),
                    ProducerMetadataValue::Object(values) => pending.extend(values.values()),
                    ProducerMetadataValue::Null
                    | ProducerMetadataValue::Bool(_)
                    | ProducerMetadataValue::String(_) => {}
                }
            }
        }
    }
    Ok(())
}
