use std::collections::BTreeMap;

use super::super::super::diagnostics::DUPLICATE_DEFINITION;
use super::super::super::raw::{
    Named, RawPresentationAffordanceFieldDefinition, RawPresentationAffordanceFieldSource,
};
use super::super::super::validate::{PendingTypeReference, validate_manifest_name};
use super::super::LoweringContext;
use super::literal::lower_literal_for_type;
use super::reference::{lower_output_type_ref, validate_ref_type};
use super::{FieldSourceContext, ProjectionBinding, ProjectionTypeTables, ProjectorContext};
use crate::DiagnosticArgumentValue;
use crate::schema::schema_diagnostic;
use crate::schema::{
    PresentationAffordanceFieldDefinition, PresentationAffordanceFieldSource, ProjectionInputRef,
};

pub(super) fn lower_fields(
    lowering: &mut LoweringContext<'_>,
    projector_context: &ProjectorContext<'_>,
    output: &str,
    raw_fields: Vec<Named<RawPresentationAffordanceFieldDefinition>>,
    types: &ProjectionTypeTables,
    pending_type_refs: &mut Vec<PendingTypeReference>,
    output_path: &[String],
) -> BTreeMap<String, PresentationAffordanceFieldDefinition> {
    let mut seen = std::collections::BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_field in raw_fields {
        let mut field_path = output_path.to_vec();
        field_path.extend(["fields".to_owned(), raw_field.name.clone()]);
        let field_span = lowering.key_span_at(&field_path, &raw_field.name);
        validate_manifest_name(
            lowering.diagnostics,
            "projection output field name",
            &raw_field.name,
            field_span.clone(),
        );
        if !seen.insert(raw_field.name.clone()) {
            lowering.diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-projection-field",
                format!(
                    "projector '{}' output '{}' repeats field '{}'",
                    projector_context.projector, output, raw_field.name
                ),
                field_span,
                [
                    (
                        "projector",
                        DiagnosticArgumentValue::String(projector_context.projector.to_owned()),
                    ),
                    ("output", DiagnosticArgumentValue::String(output.to_owned())),
                    (
                        "field",
                        DiagnosticArgumentValue::String(raw_field.name.clone()),
                    ),
                ],
            ));
            continue;
        }
        let mut type_path = field_path.clone();
        type_path.push("type".to_owned());
        let type_ref = lower_output_type_ref(
            lowering,
            ProjectionBinding {
                projector: projector_context.projector,
                output,
                name: &raw_field.name,
            },
            &raw_field.value.type_ref,
            pending_type_refs,
            &type_path,
        );
        let literal_span = match &raw_field.value.source {
            RawPresentationAffordanceFieldSource::Literal { .. } => {
                let mut value_path = field_path.clone();
                value_path.extend(["source".to_owned(), "value".to_owned()]);
                lowering.value_span_at(&value_path, "literal")
            }
            _ => field_span.clone(),
        };
        let source = lower_field_source(
            lowering,
            FieldSourceContext {
                schema: projector_context.schema,
                projector: projector_context.projector,
                output,
                field: &raw_field.name,
                type_ref: &type_ref,
                types,
                spans: super::FieldSpans {
                    span: field_span,
                    literal_span,
                },
            },
            raw_field.value.source,
        );
        lowered.insert(
            raw_field.name,
            PresentationAffordanceFieldDefinition { source, type_ref },
        );
    }
    lowered
}

fn lower_field_source(
    lowering: &mut LoweringContext<'_>,
    context: FieldSourceContext<'_>,
    raw: RawPresentationAffordanceFieldSource,
) -> PresentationAffordanceFieldSource {
    match raw {
        RawPresentationAffordanceFieldSource::Input { name } => {
            let input_ref = ProjectionInputRef::Input { name: name.clone() };
            validate_ref_type(
                lowering.diagnostics,
                super::ReferenceTypeContext {
                    projector: context.projector,
                    owner: &format!("output '{}' field '{}'", context.output, context.field),
                    expected: context.type_ref,
                    input_types: &context.types.input_types,
                    query_types: &context.types.query_types,
                },
                &input_ref,
                context.spans.span,
            );
            PresentationAffordanceFieldSource::Input { name }
        }
        RawPresentationAffordanceFieldSource::QueryResult { name } => {
            let input_ref = ProjectionInputRef::QueryResult { name: name.clone() };
            validate_ref_type(
                lowering.diagnostics,
                super::ReferenceTypeContext {
                    projector: context.projector,
                    owner: &format!("output '{}' field '{}'", context.output, context.field),
                    expected: context.type_ref,
                    input_types: &context.types.input_types,
                    query_types: &context.types.query_types,
                },
                &input_ref,
                context.spans.span,
            );
            PresentationAffordanceFieldSource::QueryResult { name }
        }
        RawPresentationAffordanceFieldSource::Literal { value } => {
            let literal = lower_literal_for_type(
                lowering.diagnostics,
                context.schema,
                &format!(
                    "projector '{}' output '{}' field '{}'",
                    context.projector, context.output, context.field
                ),
                context.type_ref,
                value,
                context.spans.literal_span,
            )
            .unwrap_or_else(|| crate::schema::SchemaLiteralValue::String(String::new()));
            PresentationAffordanceFieldSource::Literal(literal)
        }
    }
}
