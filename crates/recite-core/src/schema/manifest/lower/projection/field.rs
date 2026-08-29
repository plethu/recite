use std::collections::BTreeMap;

use super::super::super::diagnostics::DUPLICATE_DEFINITION;
use super::super::super::raw::{
    Named, RawPresentationAffordanceFieldDefinition, RawPresentationAffordanceFieldSource,
};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{PendingTypeReference, validate_manifest_name};
use super::literal::lower_literal_for_type;
use super::reference::{lower_output_type_ref, validate_ref_type};
use crate::schema::schema_diagnostic;
use crate::schema::{
    PresentationAffordanceFieldDefinition, PresentationAffordanceFieldSource, ProjectSchema,
    ProjectionInputRef, SchemaTypeRef,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_fields(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_fields: Vec<Named<RawPresentationAffordanceFieldDefinition>>,
    schema: &ProjectSchema,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
    output_path: &[String],
) -> BTreeMap<String, PresentationAffordanceFieldDefinition> {
    let mut seen = std::collections::BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_field in raw_fields {
        let mut field_path = output_path.to_vec();
        field_path.extend(["fields".to_owned(), raw_field.name.clone()]);
        let field_span = spans.key_span_at(file, source, &field_path, &raw_field.name);
        validate_manifest_name(
            diagnostics,
            "projection output field name",
            &raw_field.name,
            field_span.clone(),
        );
        if !seen.insert(raw_field.name.clone()) {
            diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-projection-field",
                format!(
                    "projector '{projector}' output '{output}' repeats field '{}'",
                    raw_field.name
                ),
                field_span,
                [
                    (
                        "projector",
                        DiagnosticArgumentValue::String(projector.to_owned()),
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
            file,
            source,
            spans,
            diagnostics,
            projector,
            output,
            &raw_field.name,
            &raw_field.value.type_ref,
            pending_type_refs,
            &type_path,
        );
        let literal_span = match &raw_field.value.source {
            RawPresentationAffordanceFieldSource::Literal { .. } => {
                let mut value_path = field_path.clone();
                value_path.extend(["source".to_owned(), "value".to_owned()]);
                spans.value_span_at(file, source, &value_path, "literal")
            }
            _ => field_span.clone(),
        };
        let source = lower_field_source(
            diagnostics,
            schema,
            projector,
            output,
            &raw_field.name,
            raw_field.value.source,
            &type_ref,
            input_types,
            query_types,
            field_span,
            literal_span,
        );
        lowered.insert(
            raw_field.name,
            PresentationAffordanceFieldDefinition { source, type_ref },
        );
    }
    lowered
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared schema and type context"
)]
fn lower_field_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    output: &str,
    field: &str,
    raw: RawPresentationAffordanceFieldSource,
    type_ref: &SchemaTypeRef,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    span: crate::SourceSpan,
    literal_span: crate::SourceSpan,
) -> PresentationAffordanceFieldSource {
    match raw {
        RawPresentationAffordanceFieldSource::Input { name } => {
            let input_ref = ProjectionInputRef::Input { name: name.clone() };
            validate_ref_type(
                diagnostics,
                projector,
                &format!("output '{output}' field '{field}'"),
                &input_ref,
                type_ref,
                input_types,
                query_types,
                span,
            );
            PresentationAffordanceFieldSource::Input { name }
        }
        RawPresentationAffordanceFieldSource::QueryResult { name } => {
            let input_ref = ProjectionInputRef::QueryResult { name: name.clone() };
            validate_ref_type(
                diagnostics,
                projector,
                &format!("output '{output}' field '{field}'"),
                &input_ref,
                type_ref,
                input_types,
                query_types,
                span,
            );
            PresentationAffordanceFieldSource::QueryResult { name }
        }
        RawPresentationAffordanceFieldSource::Literal { value } => {
            let literal = lower_literal_for_type(
                diagnostics,
                schema,
                &format!("projector '{projector}' output '{output}' field '{field}'"),
                type_ref,
                value,
                literal_span,
            )
            .unwrap_or_else(|| crate::schema::SchemaLiteralValue::String(String::new()));
            PresentationAffordanceFieldSource::Literal(literal)
        }
    }
}
