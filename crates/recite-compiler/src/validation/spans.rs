use recite_core::{
    Argument, ConditionCall, ConditionExpression, SourceFile, SourceMetadataEntry, SourceSpan,
    SourceText,
};

use super::metadata::MetadataValidationContext;
use super::participation::{ValidationCompleteness, ValidationParticipation};
use super::project;
use super::state::Validator;
use super::values::{argument_has_non_finite_float, source_metadata_value_has_non_finite_float};
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_source_text(
        &mut self,
        source_file: &'a SourceFile,
        source_text: &'a SourceText,
        owner: diagnostics::SourceSpanOwner,
        participation: ValidationParticipation,
    ) {
        self.validate_span(source_file, &source_text.span, owner);
        if participation.inline_markup == ValidationCompleteness::Complete {
            self.validate_markup(source_text);
        }
    }
    pub(super) fn validate_metadata(
        &mut self,
        source_file: &'a SourceFile,
        context: MetadataValidationContext<'a>,
    ) {
        for entry in context.metadata {
            if let Some(span) = &entry.source_span {
                self.validate_span(
                    source_file,
                    span,
                    diagnostics::SourceSpanOwner::MetadataEntry,
                );
            }
            if let Some(span) = &entry.key_span {
                self.validate_span(source_file, span, diagnostics::SourceSpanOwner::MetadataKey);
            }
            if let Some(span) = &entry.value_span {
                self.validate_span(
                    source_file,
                    span,
                    diagnostics::SourceSpanOwner::MetadataValue,
                );
            }
            self.validate_metadata_value(source_file, entry);
        }
        self.validate_metadata_schema(source_file, context);
    }
    pub(super) fn validate_condition_expression(
        &mut self,
        source_file: &'a SourceFile,
        condition: &'a ConditionExpression,
    ) {
        match condition {
            ConditionExpression::Call(call) => self.validate_condition_call(source_file, call),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                self.validate_span(
                    source_file,
                    &group.span,
                    diagnostics::SourceSpanOwner::ConditionExpression,
                );
                for expression in &group.expressions {
                    self.validate_condition_expression(source_file, expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.validate_span(
                    source_file,
                    &unary.span,
                    diagnostics::SourceSpanOwner::ConditionExpression,
                );
                self.validate_condition_expression(source_file, &unary.expression);
            }
        }
    }
    pub(super) fn validate_condition_call(
        &mut self,
        source_file: &'a SourceFile,
        call: &'a ConditionCall,
    ) {
        self.validate_span(
            source_file,
            &call.span,
            diagnostics::SourceSpanOwner::ConditionCall,
        );
        if let Some(span) = &call.function_span {
            self.validate_span(
                source_file,
                span,
                diagnostics::SourceSpanOwner::ConditionFunction,
            );
        }
        for span in &call.arg_spans {
            self.validate_span(
                source_file,
                span,
                diagnostics::SourceSpanOwner::ConditionArgument,
            );
        }
        self.validate_arguments(
            &call.args,
            call.span.clone(),
            diagnostics::ArgumentOwner::Condition,
        );
    }
    pub(super) fn validate_metadata_value(
        &mut self,
        source_file: &'a SourceFile,
        entry: &'a SourceMetadataEntry,
    ) {
        if !source_metadata_value_has_non_finite_float(&entry.value) {
            return;
        }

        let span = entry
            .value_span
            .clone()
            .or_else(|| entry.source_span.clone())
            .unwrap_or_else(|| project::first_source_span(&[source_file]));
        self.diagnostics.push(diagnostics::non_finite_float_value(
            span,
            diagnostics::NonFiniteFloatOwner::MetadataValue(entry.key.clone()),
        ));
    }
    pub(super) fn validate_arguments(
        &mut self,
        arguments: &'a [Argument],
        span: SourceSpan,
        owner: diagnostics::ArgumentOwner,
    ) {
        if arguments.iter().any(argument_has_non_finite_float) {
            self.diagnostics.push(diagnostics::non_finite_float_value(
                span,
                match owner {
                    diagnostics::ArgumentOwner::Condition => {
                        diagnostics::NonFiniteFloatOwner::ConditionArgument
                    }
                    diagnostics::ArgumentOwner::Effect => {
                        diagnostics::NonFiniteFloatOwner::EffectArgument
                    }
                },
            ));
        }
    }
    pub(super) fn validate_span(
        &mut self,
        source_file: &'a SourceFile,
        span: &SourceSpan,
        owner: diagnostics::SourceSpanOwner,
    ) {
        if span.file != source_file.path {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                diagnostics::SourceSpanError::FileMismatch,
            ));
        }

        if span.end.is_some_and(|end| end < span.start) {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                diagnostics::SourceSpanError::EndPrecedesStart,
            ));
        }
    }
}
