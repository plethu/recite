use recite_runtime::{ConditionValue, PreviewConditionArgument, PreviewConditionQuery};

use crate::{BenchmarkResult, error};

use super::RuntimeFixture;

impl RuntimeFixture {
    pub fn preview_condition_answer(
        &self,
        query: &PreviewConditionQuery,
    ) -> BenchmarkResult<ConditionValue> {
        let key = preview_condition_key(query);
        let Some(value) = self.conditions.get(&key).cloned() else {
            return Err(error(format!(
                "benchmark fixture has no preview condition value for `{key}`"
            )));
        };
        if value.kind() != query.expected_type() {
            return Err(error(format!(
                "preview condition `{key}` returned {:?}, expected {:?}",
                value.kind(),
                query.expected_type()
            )));
        }
        Ok(value)
    }
}

fn preview_condition_key(query: &PreviewConditionQuery) -> String {
    let args = query
        .arguments()
        .iter()
        .map(format_preview_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({args})", query.function())
}

fn format_preview_argument(argument: &PreviewConditionArgument) -> String {
    match argument {
        PreviewConditionArgument::Identifier(value) => value.clone(),
        PreviewConditionArgument::String(value) => format!("\"{value}\""),
        PreviewConditionArgument::Integer(value) => value.to_string(),
        PreviewConditionArgument::Float(value) => value.to_string(),
        PreviewConditionArgument::Boolean(value) => value.to_string(),
        _ => format!("{argument:?}"),
    }
}
