use std::collections::BTreeSet;

use super::super::raw::RawParameterDefinition;
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, validate_manifest_name, validate_non_empty_string,
};
use crate::schema::{ParameterDefinition, SchemaTypeRef, schema_diagnostic};
use crate::{Diagnostic, DiagnosticArgumentValue};

#[expect(
    clippy::too_many_arguments,
    reason = "parameter lowering carries shared span, validation, and semantic path context"
)]
pub(super) fn lower_params_at(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    params: &[RawParameterDefinition],
    pending_type_refs: &mut Vec<PendingTypeReference>,
    parent_path: &[String],
) -> Vec<ParameterDefinition> {
    let mut seen = BTreeSet::new();
    params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let mut param_path = parent_path.to_vec();
            param_path.push("params".to_owned());
            param_path.push(format!("[{index}]"));
            let mut name_path = param_path.clone();
            name_path.push("name".to_owned());
            let name_span = spans.value_span_at(file, source, &name_path, &param.name);
            if validate_non_empty_string(
                diagnostics,
                "parameter name",
                &param.name,
                name_span.clone(),
            ) {
                validate_manifest_name(
                    diagnostics,
                    "parameter name",
                    &param.name,
                    name_span.clone(),
                );
            }
            if !seen.insert(param.name.clone()) {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::DUPLICATE_DEFINITION,
                    "diagnostic-schema-003-parameter",
                    format!("{owner} repeats parameter '{}'", param.name),
                    name_span,
                    [
                        ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
                        (
                            "parameter",
                            DiagnosticArgumentValue::String(param.name.clone()),
                        ),
                    ],
                ));
            }

            let mut type_path = param_path;
            type_path.push("type".to_owned());
            let (mut type_ref, type_ref_span, type_ref_is_valid) =
                super::types::lower_type_reference_at_with_context(
                    file,
                    source,
                    spans,
                    diagnostics,
                    &param.type_ref,
                    &type_path,
                    format!(
                        "parameter '{}' has invalid type reference '{}'",
                        param.name, param.type_ref
                    ),
                    super::types::TypeReferenceContext::Parameter {
                        parameter: param.name.clone(),
                    },
                );
            let type_ref_is_valid = type_ref_is_valid
                && validate_parameter_type_ref(
                    diagnostics,
                    owner,
                    param,
                    &mut type_ref,
                    &type_ref_span,
                );
            if type_ref_is_valid {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("{owner} parameter '{}'", param.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }

            ParameterDefinition {
                name: param.name.clone(),
                type_ref,
            }
        })
        .collect()
}

fn validate_parameter_type_ref(
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    param: &RawParameterDefinition,
    type_ref: &mut SchemaTypeRef,
    type_ref_span: &crate::SourceSpan,
) -> bool {
    if !contains_symbol_type_ref(type_ref)
        && (!matches!(type_ref, SchemaTypeRef::Array(_))
            || owner.starts_with("projection query function "))
    {
        return true;
    }

    diagnostics.push(schema_diagnostic(
        super::super::diagnostics::INVALID_TYPE_REFERENCE,
        "diagnostic-schema-004-parameter-special-type",
        format!(
            "{owner} parameter '{}' uses projection-only or metadata-only type reference '{}'",
            param.name,
            type_ref_name(type_ref)
        ),
        type_ref_span.clone(),
        [
            ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
            (
                "parameter",
                DiagnosticArgumentValue::String(param.name.clone()),
            ),
            (
                "type_ref",
                DiagnosticArgumentValue::String(type_ref_name(type_ref)),
            ),
        ],
    ));
    *type_ref = SchemaTypeRef::String;
    false
}

fn contains_symbol_type_ref(type_ref: &SchemaTypeRef) -> bool {
    match type_ref {
        SchemaTypeRef::Symbol => true,
        SchemaTypeRef::Array(inner) => contains_symbol_type_ref(inner),
        _ => false,
    }
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
