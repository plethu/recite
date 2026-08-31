use recite_runtime::{
    ConditionValue, PreviewConditionArgument, PreviewConditionRequest, PreviewInputRevision,
};

use super::fixture::{FixtureConditionValue, RuntimeFixture};
use super::trace::{TraceScalar, condition_query_text};

pub(super) fn make_inputs_revision() -> PreviewInputRevision {
    // Fixture data and its provider are loaded once per command and remain immutable during a run.
    PreviewInputRevision::new(0)
}

pub(super) fn condition_answer(
    fixture: &RuntimeFixture,
    request: &PreviewConditionRequest,
) -> Result<recite_runtime::ConditionAnswer, crate::error::CliError> {
    let arguments = request
        .query()
        .arguments()
        .iter()
        .map(trace_preview_condition_argument)
        .collect::<Result<Vec<_>, _>>()?;
    let query = condition_query_text(request.query().function(), &arguments);
    let Some(value) = fixture.conditions.get(&query) else {
        return Ok(recite_runtime::ConditionAnswer::Failed {
            reason: format!("fixture is missing condition `{query}`"),
        });
    };
    let answer = match (request.query().expected_type(), value) {
        (recite_runtime::ConditionExpectedType::Bool, FixtureConditionValue::Bool(value)) => {
            recite_runtime::ConditionAnswer::Value(ConditionValue::Bool(*value))
        }
        (recite_runtime::ConditionExpectedType::Enum, FixtureConditionValue::Enum { r#enum }) => {
            recite_runtime::ConditionAnswer::Value(ConditionValue::EnumVariant(r#enum.clone()))
        }
        (recite_runtime::ConditionExpectedType::Bool, FixtureConditionValue::Enum { r#enum }) => {
            recite_runtime::ConditionAnswer::Value(ConditionValue::EnumVariant(r#enum.clone()))
        }
        (recite_runtime::ConditionExpectedType::Enum, FixtureConditionValue::Bool(value)) => {
            recite_runtime::ConditionAnswer::Value(ConditionValue::Bool(*value))
        }
    };
    Ok(answer)
}

fn trace_preview_condition_argument(
    argument: &PreviewConditionArgument,
) -> Result<TraceScalar, crate::error::CliError> {
    let value = match argument {
        PreviewConditionArgument::Identifier(value) => TraceScalar::Identifier(value.clone()),
        PreviewConditionArgument::String(value) => TraceScalar::String(value.clone()),
        PreviewConditionArgument::Integer(value) => TraceScalar::Integer(*value),
        PreviewConditionArgument::Float(value) => TraceScalar::Float(*value),
        PreviewConditionArgument::Boolean(value) => TraceScalar::Boolean(*value),
        _ => return Err(crate::error::CliError::UnsupportedPreviewArgument),
    };
    Ok(value)
}
