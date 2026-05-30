use std::collections::BTreeMap;

use recite_core::{ChoiceId, LocaleId};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext,
};
use serde::Deserialize;

use crate::{BenchmarkResult, error};

#[derive(Clone, Debug)]
pub struct RuntimeFixture {
    locale: LocaleId,
    catalogs: BTreeMap<String, Vec<String>>,
    conditions: BTreeMap<String, ConditionValue>,
    choices: BTreeMap<String, ChoiceId>,
    auto_ack_blocking: bool,
}

impl RuntimeFixture {
    pub fn load(source: &str) -> BenchmarkResult<Self> {
        let raw = toml::from_str::<RawRuntimeFixture>(source)?;
        let locale = LocaleId::new(raw.dialogue.locale)?;
        let mut conditions = BTreeMap::new();
        for (key, value) in raw.conditions {
            conditions.insert(key, value.into_condition_value());
        }
        let mut choices = BTreeMap::new();
        for (line, choice) in raw.choices {
            choices.insert(line, ChoiceId::new(choice)?);
        }
        Ok(Self {
            locale,
            catalogs: raw.dialogue.catalogs,
            conditions,
            choices,
            auto_ack_blocking: raw.effects.auto_ack_blocking,
        })
    }

    #[must_use]
    pub fn locale(&self) -> LocaleId {
        self.locale.clone()
    }

    #[must_use]
    pub fn catalogs(&self) -> &BTreeMap<String, Vec<String>> {
        &self.catalogs
    }

    pub fn choice_for_line(&self, line_id: &str) -> BenchmarkResult<ChoiceId> {
        self.choices.get(line_id).cloned().ok_or_else(|| {
            error(format!(
                "runtime fixture has no choice for line `{line_id}`"
            ))
        })
    }

    #[must_use]
    pub fn auto_ack_blocking(&self) -> bool {
        self.auto_ack_blocking
    }
}

impl DialogueContext for RuntimeFixture {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let key = condition_key(query);
        let Some(value) = self.conditions.get(&key) else {
            return Err(ConditionEvaluationError::new(format!(
                "benchmark fixture has no condition value for `{key}`"
            )));
        };
        if value.kind() != query.expected_type() {
            return Err(ConditionEvaluationError::new(format!(
                "condition `{key}` returned {:?}, expected {:?}",
                value.kind(),
                query.expected_type()
            )));
        }
        Ok(value.clone())
    }
}

#[derive(Debug, Deserialize)]
struct RawRuntimeFixture {
    dialogue: RawDialogueFixture,
    #[serde(default)]
    conditions: BTreeMap<String, RawConditionValue>,
    #[serde(default)]
    choices: BTreeMap<String, String>,
    #[serde(default)]
    effects: RawEffectsFixture,
}

#[derive(Debug, Deserialize)]
struct RawDialogueFixture {
    locale: String,
    #[serde(default)]
    catalogs: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct RawEffectsFixture {
    #[serde(default)]
    auto_ack_blocking: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawConditionValue {
    Bool(bool),
    Enum { r#enum: String },
}

impl RawConditionValue {
    fn into_condition_value(self) -> ConditionValue {
        match self {
            Self::Bool(value) => ConditionValue::Bool(value),
            Self::Enum { r#enum } => ConditionValue::EnumVariant(r#enum),
        }
    }
}

fn condition_key(query: ConditionQuery<'_>) -> String {
    let args = query
        .arguments()
        .into_iter()
        .map(format_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({args})", query.function())
}

fn format_argument(argument: ConditionArgument<'_>) -> String {
    match argument {
        ConditionArgument::Identifier(value) => value.to_owned(),
        ConditionArgument::String(value) => format!("\"{value}\""),
        ConditionArgument::Integer(value) => value.to_string(),
        ConditionArgument::Float(value) => value.to_string(),
        ConditionArgument::Boolean(value) => value.to_string(),
    }
}

#[cfg(test)]
mod tests;
