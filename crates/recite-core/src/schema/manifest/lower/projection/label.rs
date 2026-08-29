use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{
    Named, RawPresentationLabelArgDefinition, RawPresentationLabelDefinition,
};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::validate_manifest_name;
use super::reference::{lower_input_ref, lower_output_type_ref, validate_ref_type};
use crate::schema::schema_diagnostic;
use crate::schema::{PresentationLabelArgDefinition, PresentationLabelDefinition, SchemaTypeRef};
use crate::{
    Diagnostic, DiagnosticArgumentValue, PlaceholderSyntaxKind, SourceSpan,
    extract_placeholder_names,
};

macro_rules! text_args {
    ($($name:literal => $value:expr),* $(,)?) => {
        [$(($name, DiagnosticArgumentValue::String($value.into()))),*]
    };
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_label(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_label: RawPresentationLabelDefinition,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    seen_label_ids: &mut BTreeSet<String>,
    pending_type_refs: &mut Vec<super::super::super::validate::PendingTypeReference>,
    output_path: &[String],
) -> PresentationLabelDefinition {
    let mut label_path = output_path.to_vec();
    label_path.push("label".to_owned());
    let mut template_id_path = label_path.clone();
    template_id_path.push("template_id".to_owned());
    let template_id_span =
        spans.value_span_at(file, source, &template_id_path, &raw_label.template_id);
    validate_manifest_name(
        diagnostics,
        "presentation label template id",
        &raw_label.template_id,
        template_id_span,
    );
    if !seen_label_ids.insert(raw_label.template_id.clone()) {
        diagnostics.push(schema_diagnostic(
            DUPLICATE_DEFINITION,
            "diagnostic-schema-003-label-template",
            format!(
                "duplicate presentation label template id '{}'",
                raw_label.template_id
            ),
            spans.value_span_at(file, source, &template_id_path, &raw_label.template_id),
            text_args!("template_id" => raw_label.template_id.clone()),
        ));
    }
    let mut source_text_path = label_path.clone();
    source_text_path.push("source_text".to_owned());
    let source_text_span =
        spans.value_span_at(file, source, &source_text_path, &raw_label.source_text);
    super::super::super::validate::validate_non_empty_string(
        diagnostics,
        "presentation label source text",
        &raw_label.source_text,
        source_text_span.clone(),
    );
    let args = lower_label_args(
        file,
        source,
        spans,
        diagnostics,
        projector,
        output,
        raw_label.args,
        input_types,
        query_types,
        pending_type_refs,
        &label_path,
    );
    validate_label_placeholders(
        diagnostics,
        projector,
        output,
        &raw_label.template_id,
        &raw_label.source_text,
        &args,
        source_text_span,
    );
    PresentationLabelDefinition {
        template_id: raw_label.template_id,
        source_text: raw_label.source_text,
        args,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_label_args(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_args: Vec<Named<RawPresentationLabelArgDefinition>>,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    pending_type_refs: &mut Vec<super::super::super::validate::PendingTypeReference>,
    label_path: &[String],
) -> BTreeMap<String, PresentationLabelArgDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_arg in raw_args {
        let mut arg_path = label_path.to_vec();
        arg_path.extend(["args".to_owned(), raw_arg.name.clone()]);
        let arg_span = spans.key_span_at(file, source, &arg_path, &raw_arg.name);
        validate_manifest_name(
            diagnostics,
            "presentation label argument name",
            &raw_arg.name,
            arg_span.clone(),
        );
        if !seen.insert(raw_arg.name.clone()) {
            diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-label-argument",
                format!(
                    "projector '{projector}' output '{output}' repeats label argument '{}'",
                    raw_arg.name
                ),
                arg_span,
            text_args!("projector" => projector, "output" => output, "argument" => raw_arg.name.clone()),
            ));
            continue;
        }
        let mut type_path = arg_path.clone();
        type_path.push("type".to_owned());
        let type_ref = lower_output_type_ref(
            file,
            source,
            spans,
            diagnostics,
            projector,
            output,
            &raw_arg.name,
            &raw_arg.value.type_ref,
            pending_type_refs,
            &type_path,
        );
        let source_ref = lower_input_ref(raw_arg.value.source);
        validate_ref_type(
            diagnostics,
            projector,
            &format!("output '{output}' label argument '{}'", raw_arg.name),
            &source_ref,
            &type_ref,
            input_types,
            query_types,
            arg_span,
        );
        lowered.insert(
            raw_arg.name,
            PresentationLabelArgDefinition {
                source: source_ref,
                type_ref,
            },
        );
    }
    lowered
}

fn validate_label_placeholders(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    template_id: &str,
    source_text: &str,
    args: &BTreeMap<String, PresentationLabelArgDefinition>,
    span: SourceSpan,
) {
    let placeholders = match extract_placeholder_names(source_text) {
        Ok(placeholders) => placeholders,
        Err(error) => {
            let (presentation_id, mut arguments) = match error.kind() {
                PlaceholderSyntaxKind::Unterminated => (
                    "diagnostic-schema-001-label-placeholder-unterminated",
                    Vec::new(),
                ),
                PlaceholderSyntaxKind::InvalidName(name) => (
                    "diagnostic-schema-001-label-placeholder-invalid-name",
                    vec![("name", DiagnosticArgumentValue::String(name.clone()))],
                ),
                PlaceholderSyntaxKind::UnescapedClosingBrace => (
                    "diagnostic-schema-001-label-placeholder-unescaped-closing-brace",
                    Vec::new(),
                ),
            };
            arguments.extend(text_args!("projector" => projector, "output" => output, "template_id" => template_id));
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                presentation_id,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' has invalid placeholder syntax: {}",
                    error.message()
                ),
                span,
                arguments,
            ));
            return;
        }
    };
    for placeholder in &placeholders {
        if !args.contains_key(placeholder) {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-label-unknown-arg",
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' references unknown argument '{placeholder}'"
                ),
                span.clone(),
                text_args!("projector" => projector, "output" => output, "template_id" => template_id, "placeholder" => placeholder.clone()),
            ));
        }
    }
    for arg in args.keys() {
        if !placeholders.contains(arg) {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-label-unused-arg",
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' argument '{arg}' is not used in its template"
                ),
                span.clone(),
                text_args!("projector" => projector, "output" => output, "template_id" => template_id, "arg" => arg.clone()),
            ));
        }
    }
}
