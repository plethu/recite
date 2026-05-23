use recite_core::{
    Argument, Effect, EffectDefinition, ScalarValue, SchemaTypeRef, SourceFile, SourceSpan,
};

use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_effect_schema(
        &mut self,
        _source_file: &'a SourceFile,
        effect: &'a Effect,
    ) {
        let Some(schema) = self.schema else {
            return;
        };

        let Some(definition) = schema.effects.get(effect.function.as_str()) else {
            self.diagnostics.push(diagnostics::unknown_effect_function(
                &effect.function,
                self.effect_function_span(effect),
            ));
            return;
        };

        if !definition.modes.contains(&effect.mode) {
            self.diagnostics.push(diagnostics::unsupported_effect_mode(
                &effect.function,
                effect.mode,
                self.effect_mode_span(effect),
            ));
        }

        self.validate_effect_arity(effect, definition);
        for (index, (argument, parameter)) in
            effect.args.iter().zip(definition.params.iter()).enumerate()
        {
            self.validate_effect_argument(effect, index, argument, &parameter.type_ref);
        }
    }

    fn validate_effect_arity(&mut self, effect: &Effect, definition: &EffectDefinition) {
        let actual = effect.args.len();
        let expected = definition.params.len();
        if actual == expected {
            return;
        }

        let span = if actual > expected {
            self.effect_arg_span(effect, expected)
        } else {
            self.effect_call_span(effect)
        };
        self.diagnostics.push(diagnostics::wrong_effect_arity(
            &effect.function,
            expected,
            actual,
            span,
        ));
    }

    fn validate_effect_argument(
        &mut self,
        effect: &Effect,
        index: usize,
        argument: &Argument,
        type_ref: &SchemaTypeRef,
    ) {
        let span = self.effect_arg_span(effect, index);
        let Some(schema) = self.schema else {
            return;
        };

        let valid = match type_ref {
            SchemaTypeRef::String => matches!(argument, Argument::Value(ScalarValue::String(_))),
            SchemaTypeRef::Int => matches!(argument, Argument::Value(ScalarValue::Integer(_))),
            SchemaTypeRef::Float => matches!(argument, Argument::Value(ScalarValue::Float(_))),
            SchemaTypeRef::Bool => matches!(argument, Argument::Value(ScalarValue::Boolean(_))),
            SchemaTypeRef::Speaker => {
                let Some(value) =
                    self.effect_reference_argument_value(effect, index, argument, type_ref, &span)
                else {
                    return;
                };
                if !schema.speakers.contains_key(value) {
                    self.invalid_effect_argument_value(effect, index, type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Enum(enum_name) => {
                let Some(value) =
                    self.effect_reference_argument_value(effect, index, argument, type_ref, &span)
                else {
                    return;
                };
                let Some(definition) = schema.types.get(enum_name) else {
                    self.wrong_effect_argument_type(effect, index, type_ref, argument, span);
                    return;
                };
                let recite_core::SchemaTypeDefinition::Enum(definition) = definition;
                if !definition.values.contains(value) {
                    self.invalid_effect_argument_value(effect, index, type_ref, value, span);
                }
                return;
            }
            SchemaTypeRef::Registry(registry_name) => {
                let Some(value) =
                    self.effect_reference_argument_value(effect, index, argument, type_ref, &span)
                else {
                    return;
                };
                let Some(registry) = schema.registries.get(registry_name) else {
                    self.wrong_effect_argument_type(effect, index, type_ref, argument, span);
                    return;
                };
                if !registry.values.contains(value) {
                    self.invalid_effect_argument_value(effect, index, type_ref, value, span);
                }
                return;
            }
        };

        if !valid {
            self.wrong_effect_argument_type(effect, index, type_ref, argument, span);
        }
    }

    fn effect_reference_argument_value<'b>(
        &mut self,
        effect: &Effect,
        index: usize,
        argument: &'b Argument,
        type_ref: &SchemaTypeRef,
        span: &SourceSpan,
    ) -> Option<&'b str> {
        let value = argument_reference_value(argument);
        if value.is_none() {
            self.wrong_effect_argument_type(effect, index, type_ref, argument, span.clone());
        }
        value
    }

    fn wrong_effect_argument_type(
        &mut self,
        effect: &Effect,
        index: usize,
        type_ref: &SchemaTypeRef,
        argument: &Argument,
        span: SourceSpan,
    ) {
        self.diagnostics
            .push(diagnostics::wrong_effect_argument_type(
                &effect.function,
                index,
                type_ref,
                display_argument_type(argument),
                span,
            ));
    }

    fn invalid_effect_argument_value(
        &mut self,
        effect: &Effect,
        index: usize,
        type_ref: &SchemaTypeRef,
        value: &str,
        span: SourceSpan,
    ) {
        self.diagnostics
            .push(diagnostics::invalid_effect_argument_value(
                &effect.function,
                index,
                type_ref,
                value,
                span,
            ));
    }

    fn effect_mode_span(&self, effect: &Effect) -> SourceSpan {
        effect
            .mode_span
            .clone()
            .unwrap_or_else(|| effect.span.clone())
    }

    fn effect_function_span(&self, effect: &Effect) -> SourceSpan {
        effect
            .function_span
            .clone()
            .unwrap_or_else(|| effect.span.clone())
    }

    fn effect_call_span(&self, effect: &Effect) -> SourceSpan {
        effect
            .call_span
            .clone()
            .unwrap_or_else(|| effect.span.clone())
    }

    fn effect_arg_span(&self, effect: &Effect, index: usize) -> SourceSpan {
        effect
            .arg_spans
            .get(index)
            .cloned()
            .unwrap_or_else(|| effect.span.clone())
    }
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
