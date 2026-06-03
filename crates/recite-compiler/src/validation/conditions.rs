use recite_core::{
    Argument, Choice, ConditionCall, ConditionDefinition, ConditionExpression, ConditionReturnType,
    ScalarValue, SchemaTypeDefinition, SchemaTypeRef, SourceSpan,
};

use super::state::Validator;
use crate::diagnostics;

impl Validator<'_> {
    pub(super) fn validate_boolean_condition_schema(&mut self, condition: &ConditionExpression) {
        match condition {
            ConditionExpression::Call(call) => self.validate_condition_call_schema(call, true),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                for expression in &group.expressions {
                    self.validate_boolean_condition_schema(expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.validate_boolean_condition_schema(&unary.expression);
            }
        }
    }

    pub(super) fn validate_match_scrutinee_schema(&mut self, call: &ConditionCall) {
        self.validate_condition_call_schema(call, false);
    }

    pub(super) fn validate_choice_availability_reason(&mut self, choice: &Choice) {
        let Some(reason) = &choice.availability_reason_override else {
            return;
        };

        if choice.availability_requirement.is_none() {
            self.diagnostics
                .push(diagnostics::availability_reason_without_requirement(
                    reason.reason_id.as_str(),
                    reason.span.clone(),
                ));
        }

        if let Some(argument_span) = &reason.argument_span {
            self.diagnostics
                .push(diagnostics::parameterized_availability_reason_override(
                    reason.reason_id.as_str(),
                    argument_span.clone(),
                ));
        }

        let Some(schema) = self.schema else {
            return;
        };
        let Some(definition) = schema.availability_reasons.get(&reason.reason_id) else {
            self.diagnostics
                .push(diagnostics::unknown_availability_reason_override(
                    reason.reason_id.as_str(),
                    reason.id_span.clone(),
                ));
            return;
        };

        if !definition.params.is_empty() {
            self.diagnostics
                .push(diagnostics::parameterized_availability_reason_override(
                    reason.reason_id.as_str(),
                    reason.id_span.clone(),
                ));
        }
    }

    fn validate_condition_call_schema(&mut self, call: &ConditionCall, requires_bool: bool) {
        let Some(schema) = self.schema else {
            return;
        };

        let Some(definition) = schema.conditions.get(call.function.as_str()) else {
            self.diagnostics
                .push(diagnostics::unknown_condition_function(
                    &call.function,
                    condition_function_span(call),
                ));
            return;
        };
        let definition = definition.clone();

        self.validate_condition_arity(call, &definition);
        for (index, (argument, parameter)) in
            call.args.iter().zip(definition.params.iter()).enumerate()
        {
            self.validate_condition_argument(call, index, argument, &parameter.type_ref);
        }

        match (&definition.returns, requires_bool) {
            (ConditionReturnType::Bool, true) | (ConditionReturnType::Enum(_), false) => {}
            (actual, true) => self
                .diagnostics
                .push(diagnostics::wrong_condition_return_type(
                    &call.function,
                    "bool",
                    actual,
                    condition_function_span(call),
                )),
            (actual, false) => self
                .diagnostics
                .push(diagnostics::wrong_condition_return_type(
                    &call.function,
                    "enum",
                    actual,
                    condition_function_span(call),
                )),
        }
    }

    fn validate_condition_arity(&mut self, call: &ConditionCall, definition: &ConditionDefinition) {
        let actual = call.args.len();
        let expected = definition.params.len();
        if actual == expected {
            return;
        }

        let span = if actual > expected {
            condition_arg_span(call, expected)
        } else {
            call.span.clone()
        };
        self.diagnostics.push(diagnostics::wrong_condition_arity(
            &call.function,
            expected,
            actual,
            span,
        ));
    }

    fn validate_condition_argument(
        &mut self,
        call: &ConditionCall,
        index: usize,
        argument: &Argument,
        type_ref: &SchemaTypeRef,
    ) {
        let span = condition_arg_span(call, index);
        let Some(schema) = self.schema else {
            return;
        };

        let valid = match type_ref {
            SchemaTypeRef::String => matches!(argument, Argument::Value(ScalarValue::String(_))),
            SchemaTypeRef::Symbol => matches!(argument, Argument::Identifier(_)),
            SchemaTypeRef::Int => matches!(argument, Argument::Value(ScalarValue::Integer(_))),
            SchemaTypeRef::Float => matches!(argument, Argument::Value(ScalarValue::Float(_))),
            SchemaTypeRef::Bool => matches!(argument, Argument::Value(ScalarValue::Boolean(_))),
            SchemaTypeRef::Speaker => {
                let Some(value) =
                    self.condition_reference_argument_value(call, index, argument, type_ref, &span)
                else {
                    return;
                };
                if !schema.speakers.contains_key(value) {
                    self.invalid_condition_argument_value(call, index, type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Enum(enum_name) => {
                let Some(value) =
                    self.condition_reference_argument_value(call, index, argument, type_ref, &span)
                else {
                    return;
                };
                let Some(definition) = schema.types.get(enum_name) else {
                    self.wrong_condition_argument_type(call, index, type_ref, argument, span);
                    return;
                };
                let SchemaTypeDefinition::Enum(definition) = definition;
                if !definition.values.contains(value) {
                    self.invalid_condition_argument_value(call, index, type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Registry(registry_name) => {
                let Some(value) =
                    self.condition_reference_argument_value(call, index, argument, type_ref, &span)
                else {
                    return;
                };
                let Some(registry) = schema.registries.get(registry_name) else {
                    self.wrong_condition_argument_type(call, index, type_ref, argument, span);
                    return;
                };
                if !registry.values.contains(value) {
                    self.invalid_condition_argument_value(call, index, type_ref, value, span);
                }
                return;
            }
        };

        if !valid {
            self.wrong_condition_argument_type(call, index, type_ref, argument, span);
        }
    }

    fn condition_reference_argument_value<'b>(
        &mut self,
        call: &ConditionCall,
        index: usize,
        argument: &'b Argument,
        type_ref: &SchemaTypeRef,
        span: &SourceSpan,
    ) -> Option<&'b str> {
        let value = argument_reference_value(argument);
        if value.is_none() {
            self.wrong_condition_argument_type(call, index, type_ref, argument, span.clone());
        }
        value
    }

    fn wrong_condition_argument_type(
        &mut self,
        call: &ConditionCall,
        index: usize,
        type_ref: &SchemaTypeRef,
        argument: &Argument,
        span: SourceSpan,
    ) {
        self.diagnostics
            .push(diagnostics::wrong_condition_argument_type(
                &call.function,
                index,
                type_ref,
                display_argument_type(argument),
                span,
            ));
    }

    fn invalid_condition_argument_value(
        &mut self,
        call: &ConditionCall,
        index: usize,
        type_ref: &SchemaTypeRef,
        value: &str,
        span: SourceSpan,
    ) {
        self.diagnostics
            .push(diagnostics::invalid_condition_argument_value(
                &call.function,
                index,
                type_ref,
                value,
                span,
            ));
    }
}

fn condition_function_span(call: &ConditionCall) -> SourceSpan {
    call.function_span
        .clone()
        .unwrap_or_else(|| call.span.clone())
}

fn condition_arg_span(call: &ConditionCall, index: usize) -> SourceSpan {
    call.arg_spans
        .get(index)
        .cloned()
        .unwrap_or_else(|| call.span.clone())
}

fn argument_reference_value(argument: &Argument) -> Option<&str> {
    match argument {
        Argument::Identifier(value) => Some(value.as_str()),
        Argument::Value(ScalarValue::String(value)) => Some(value.as_str()),
        Argument::Value(_) => None,
    }
}

fn display_argument_type(argument: &Argument) -> &'static str {
    match argument {
        Argument::Identifier(_) => "identifier",
        Argument::Value(ScalarValue::String(_)) => "string",
        Argument::Value(ScalarValue::Integer(_)) => "int",
        Argument::Value(ScalarValue::Float(_)) => "float",
        Argument::Value(ScalarValue::Boolean(_)) => "bool",
    }
}
