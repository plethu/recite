use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{
    Named, RawPresentationLabelArgDefinition, RawPresentationLabelDefinition,
};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::validate_manifest_name;
use super::reference::{lower_input_ref, lower_output_type_ref, validate_ref_type};
use crate::schema::{PresentationLabelArgDefinition, PresentationLabelDefinition, SchemaTypeRef};
use crate::{Diagnostic, SourceSpan, extract_placeholder_names};

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
) -> PresentationLabelDefinition {
    let template_id_span = spans.next_value_span(file, source, &raw_label.template_id);
    validate_manifest_name(
        diagnostics,
        "presentation label template id",
        &raw_label.template_id,
        template_id_span,
    );
    if !seen_label_ids.insert(raw_label.template_id.clone()) {
        diagnostics.push(Diagnostic::error(
            DUPLICATE_DEFINITION,
            format!(
                "duplicate presentation label template id '{}'",
                raw_label.template_id
            ),
            spans.next_value_span(file, source, &raw_label.template_id),
        ));
    }
    let source_text_span = spans.next_value_span(file, source, &raw_label.source_text);
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
) -> BTreeMap<String, PresentationLabelArgDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_arg in raw_args {
        let arg_span = spans.next_key_span(file, source, &raw_arg.name);
        validate_manifest_name(
            diagnostics,
            "presentation label argument name",
            &raw_arg.name,
            arg_span.clone(),
        );
        if !seen.insert(raw_arg.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "projector '{projector}' output '{output}' repeats label argument '{}'",
                    raw_arg.name
                ),
                arg_span,
            ));
            continue;
        }
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
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' has invalid placeholder syntax: {}",
                    error.message()
                ),
                span,
            ));
            return;
        }
    };
    for placeholder in &placeholders {
        if !args.contains_key(placeholder) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' references unknown argument '{placeholder}'"
                ),
                span.clone(),
            ));
        }
    }
    for arg in args.keys() {
        if !placeholders.contains(arg) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' argument '{arg}' is not used in its template"
                ),
                span.clone(),
            ));
        }
    }
}
