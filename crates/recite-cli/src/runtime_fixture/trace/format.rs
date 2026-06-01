use recite_runtime::{DialogueEffectArgument, DialogueEffectMode};

use crate::runtime_format::{
    RuntimeDisplayArgument, format_condition_query, format_effect_arguments as format_effect_args,
};

use super::model::TraceScalar;

pub(in crate::runtime_fixture) fn condition_query_text(
    function: &str,
    arguments: &[TraceScalar],
) -> String {
    format_condition_query(
        function,
        arguments.iter().map(trace_scalar_display_argument),
    )
}

fn trace_scalar_display_argument(argument: &TraceScalar) -> RuntimeDisplayArgument<'_> {
    match argument {
        TraceScalar::Identifier(value) => RuntimeDisplayArgument::Identifier(value),
        TraceScalar::String(value) => RuntimeDisplayArgument::String(value),
        TraceScalar::Integer(value) => RuntimeDisplayArgument::Integer(*value),
        TraceScalar::Float(value) => RuntimeDisplayArgument::Float(*value),
        TraceScalar::Boolean(value) => RuntimeDisplayArgument::Boolean(*value),
    }
}

pub(in crate::runtime_fixture) fn format_effect_arguments(
    arguments: &[DialogueEffectArgument],
) -> String {
    format_effect_args(arguments)
}

pub(super) fn effect_mode_name(mode: DialogueEffectMode) -> &'static str {
    match mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    }
}
