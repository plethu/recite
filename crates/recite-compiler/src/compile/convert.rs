use recite_core::{
    Argument, Choice, ChoiceEcho, ChoiceId, CompiledArgument, CompiledChoiceEcho,
    CompiledConditionCall, CompiledConditionExpression, CompiledEffectMode, CompiledMatchPattern,
    Effect, EffectId, EffectMode, Line, LineId, MatchPattern,
};

use super::CompileError;

pub(in crate::compile) fn required_line_id(line: &Line) -> Result<LineId, CompileError> {
    line.id.clone().ok_or_else(|| {
        CompileError::InvalidValidatedInput("validated line is missing a line ID".to_owned())
    })
}

pub(in crate::compile) fn required_choice_id(choice: &Choice) -> Result<ChoiceId, CompileError> {
    choice.id.clone().ok_or_else(|| {
        CompileError::InvalidValidatedInput("validated choice is missing a choice ID".to_owned())
    })
}

pub(in crate::compile) fn compile_choice_echo(echo: &ChoiceEcho) -> CompiledChoiceEcho {
    match echo {
        ChoiceEcho::None => CompiledChoiceEcho::None,
        ChoiceEcho::SelectedText => CompiledChoiceEcho::SelectedText,
        ChoiceEcho::Line(line_id) => CompiledChoiceEcho::ExplicitLine(line_id.clone()),
    }
}

pub(in crate::compile) fn compile_effect_mode(mode: EffectMode) -> CompiledEffectMode {
    match mode {
        EffectMode::Deferred => CompiledEffectMode::Deferred,
        EffectMode::Immediate => CompiledEffectMode::Immediate,
        EffectMode::Blocking => CompiledEffectMode::Blocking,
    }
}

pub(in crate::compile) fn compile_match_pattern(pattern: &MatchPattern) -> CompiledMatchPattern {
    match pattern {
        MatchPattern::Variant(value) => CompiledMatchPattern::Variant(value.clone()),
        MatchPattern::Wildcard => CompiledMatchPattern::Wildcard,
    }
}

pub(in crate::compile) fn compile_condition_expression(
    condition: &recite_core::ConditionExpression,
) -> CompiledConditionExpression {
    match condition {
        recite_core::ConditionExpression::Call(call) => {
            CompiledConditionExpression::Call(compile_condition_call(call))
        }
        recite_core::ConditionExpression::And(group) => CompiledConditionExpression::And(
            group
                .expressions
                .iter()
                .map(compile_condition_expression)
                .collect(),
        ),
        recite_core::ConditionExpression::Or(group) => CompiledConditionExpression::Or(
            group
                .expressions
                .iter()
                .map(compile_condition_expression)
                .collect(),
        ),
        recite_core::ConditionExpression::Not(unary) => CompiledConditionExpression::Not(Box::new(
            compile_condition_expression(&unary.expression),
        )),
        recite_core::ConditionExpression::Grouped(unary) => {
            compile_condition_expression(&unary.expression)
        }
    }
}

pub(in crate::compile) fn compile_condition_call(
    call: &recite_core::ConditionCall,
) -> CompiledConditionCall {
    CompiledConditionCall {
        function: call.function.clone(),
        args: call.args.iter().map(compile_argument).collect(),
    }
}

pub(in crate::compile) fn compile_argument(argument: &Argument) -> CompiledArgument {
    match argument {
        Argument::Identifier(value) => CompiledArgument::Identifier(value.clone()),
        Argument::Value(value) => CompiledArgument::Value(value.clone()),
    }
}

pub(in crate::compile) fn effect_id_for(effect: &Effect) -> Result<EffectId, CompileError> {
    EffectId::new(format!(
        "effect:{}:{}:{}",
        effect.span.file,
        effect.span.start.line(),
        effect.span.start.column()
    ))
    .map_err(|error| CompileError::InvalidValidatedInput(error.to_string()))
}
