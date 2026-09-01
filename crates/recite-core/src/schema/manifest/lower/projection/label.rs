use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{
    Named, RawPresentationLabelArgDefinition, RawPresentationLabelDefinition,
};
use super::super::super::validate::validate_manifest_name;
use super::super::LoweringContext;
use super::reference::{lower_input_ref, lower_output_type_ref, validate_ref_type};
use super::{LabelContext, LabelIdState, PendingTypeRefs, ProjectionBinding};
use crate::schema::schema_diagnostic;
use crate::schema::{PresentationLabelArgDefinition, PresentationLabelDefinition};
use crate::{
    Diagnostic, DiagnosticArgumentValue, PlaceholderSyntaxKind, SourceSpan,
    extract_placeholder_names,
};

macro_rules! text_args {
    ($($name:literal => $value:expr),* $(,)?) => {
        [$(($name, DiagnosticArgumentValue::String($value.into()))),*]
    };
}

pub(super) fn lower_label(
    lowering: &mut LoweringContext<'_>,
    context: &LabelContext<'_>,
    raw_label: RawPresentationLabelDefinition,
    label_ids: &mut LabelIdState<'_>,
    pending_type_refs: &mut PendingTypeRefs<'_>,
    output_path: &[String],
) -> PresentationLabelDefinition {
    let mut label_path = output_path.to_vec();
    label_path.push("label".to_owned());
    let mut template_id_path = label_path.clone();
    template_id_path.push("template_id".to_owned());
    let template_id_span = lowering.value_span_at(&template_id_path, &raw_label.template_id);
    validate_manifest_name(
        lowering.diagnostics,
        "presentation label template id",
        &raw_label.template_id,
        template_id_span,
    );
    if !label_ids
        .seen_label_ids
        .insert(raw_label.template_id.clone())
    {
        let duplicate_span = lowering.value_span_at(&template_id_path, &raw_label.template_id);
        lowering.diagnostics.push(schema_diagnostic(
            DUPLICATE_DEFINITION,
            "diagnostic-schema-003-label-template",
            format!(
                "duplicate presentation label template id '{}'",
                raw_label.template_id
            ),
            duplicate_span,
            text_args!("template_id" => raw_label.template_id.clone()),
        ));
    }
    let mut source_text_path = label_path.clone();
    source_text_path.push("source_text".to_owned());
    let source_text_span = lowering.value_span_at(&source_text_path, &raw_label.source_text);
    super::super::super::validate::validate_non_empty_string(
        lowering.diagnostics,
        "presentation label source text",
        &raw_label.source_text,
        source_text_span.clone(),
    );
    let args = lower_label_args(
        lowering,
        context,
        raw_label.args,
        pending_type_refs,
        &label_path,
    );
    validate_label_placeholders(
        lowering.diagnostics,
        context.projector,
        context.output,
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

fn lower_label_args(
    lowering: &mut LoweringContext<'_>,
    context: &LabelContext<'_>,
    raw_args: Vec<Named<RawPresentationLabelArgDefinition>>,
    pending_type_refs: &mut PendingTypeRefs<'_>,
    label_path: &[String],
) -> BTreeMap<String, PresentationLabelArgDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_arg in raw_args {
        let mut arg_path = label_path.to_vec();
        arg_path.extend(["args".to_owned(), raw_arg.name.clone()]);
        let arg_span = lowering.key_span_at(&arg_path, &raw_arg.name);
        validate_manifest_name(
            lowering.diagnostics,
            "presentation label argument name",
            &raw_arg.name,
            arg_span.clone(),
        );
        if !seen.insert(raw_arg.name.clone()) {
            lowering.diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-label-argument",
                format!(
                    "projector '{}' output '{}' repeats label argument '{}'",
                    context.projector,
                    context.output,
                    raw_arg.name
                ),
                arg_span,
            text_args!("projector" => context.projector, "output" => context.output, "argument" => raw_arg.name.clone()),
            ));
            continue;
        }
        let mut type_path = arg_path.clone();
        type_path.push("type".to_owned());
        let type_ref = lower_output_type_ref(
            lowering,
            ProjectionBinding {
                projector: context.projector,
                output: context.output,
                name: &raw_arg.name,
            },
            &raw_arg.value.type_ref,
            pending_type_refs,
            &type_path,
        );
        let source_ref = lower_input_ref(raw_arg.value.source);
        validate_ref_type(
            lowering.diagnostics,
            super::ReferenceTypeContext {
                projector: context.projector,
                owner: &format!(
                    "output '{}' label argument '{}'",
                    context.output, raw_arg.name
                ),
                expected: &type_ref,
                input_types: &context.types.input_types,
                query_types: &context.types.query_types,
            },
            &source_ref,
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
