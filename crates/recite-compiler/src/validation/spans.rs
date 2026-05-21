use recite_core::{
    Argument, ConditionCall, ConditionExpression, Metadata, MetadataEntry, SourceFile, SourceSpan,
    SourceText,
};

use super::project;
use super::state::Validator;
use super::values::{argument_has_non_finite_float, value_has_non_finite_float};
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_source_text(
        &mut self,
        source_file: &'a SourceFile,
        source_text: &'a SourceText,
        owner: &'static str,
    ) {
        self.validate_span(source_file, &source_text.span, owner);
    }
    pub(super) fn validate_metadata(
        &mut self,
        source_file: &'a SourceFile,
        metadata: &'a Metadata,
    ) {
        for entry in metadata {
            if let Some(span) = &entry.source_span {
                self.validate_span(source_file, span, "metadata entry");
            }
            if let Some(span) = &entry.key_span {
                self.validate_span(source_file, span, "metadata key");
            }
            if let Some(span) = &entry.value_span {
                self.validate_span(source_file, span, "metadata value");
            }
            self.validate_metadata_value(source_file, entry);
        }
    }
    pub(super) fn validate_condition_expression(
        &mut self,
        source_file: &'a SourceFile,
        condition: &'a ConditionExpression,
    ) {
        match condition {
            ConditionExpression::Call(call) => self.validate_condition_call(source_file, call),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                self.validate_span(source_file, &group.span, "condition expression");
                for expression in &group.expressions {
                    self.validate_condition_expression(source_file, expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.validate_span(source_file, &unary.span, "condition expression");
                self.validate_condition_expression(source_file, &unary.expression);
            }
        }
    }
    pub(super) fn validate_condition_call(
        &mut self,
        source_file: &'a SourceFile,
        call: &'a ConditionCall,
    ) {
        self.validate_span(source_file, &call.span, "condition call");
        if let Some(span) = &call.function_span {
            self.validate_span(source_file, span, "condition function");
        }
        for span in &call.arg_spans {
            self.validate_span(source_file, span, "condition argument");
        }
        self.validate_arguments(&call.args, call.span.clone(), "condition argument");
    }
    pub(super) fn validate_metadata_value(
        &mut self,
        source_file: &'a SourceFile,
        entry: &'a MetadataEntry,
    ) {
        if !value_has_non_finite_float(&entry.value) {
            return;
        }

        let span = entry
            .value_span
            .clone()
            .or_else(|| entry.source_span.clone())
            .unwrap_or_else(|| project::first_source_span(&[source_file]));
        self.diagnostics.push(diagnostics::non_finite_float_value(
            span,
            format!("metadata value `{}`", entry.key),
        ));
    }
    pub(super) fn validate_arguments(
        &mut self,
        arguments: &'a [Argument],
        span: SourceSpan,
        owner: &'static str,
    ) {
        if arguments.iter().any(argument_has_non_finite_float) {
            self.diagnostics
                .push(diagnostics::non_finite_float_value(span, owner));
        }
    }
    pub(super) fn validate_span(
        &mut self,
        source_file: &'a SourceFile,
        span: &SourceSpan,
        owner: &'static str,
    ) {
        if span.file != source_file.path {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                "span file does not match source file",
            ));
        }

        if span.end.is_some_and(|end| end < span.start) {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                "span end precedes span start",
            ));
        }
    }
}
