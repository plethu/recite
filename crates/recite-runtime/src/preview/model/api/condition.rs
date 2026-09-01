use recite_core::BlockId;

use crate::{ConditionArgument, ConditionValue};

use super::prompt::PreviewPromptIdentity;
use super::{ConditionAnswer, PreviewConditionRequestId};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewConditionRequest {
    pub(crate) id: PreviewConditionRequestId,
    pub(crate) block: BlockId,
    pub(crate) prompt: Option<PreviewPromptIdentity>,
    pub(crate) query: PreviewConditionQuery,
}

impl PreviewConditionRequest {
    #[must_use]
    pub fn id(&self) -> PreviewConditionRequestId {
        self.id
    }

    #[must_use]
    pub fn block(&self) -> &BlockId {
        &self.block
    }

    #[must_use]
    pub fn prompt(&self) -> Option<&PreviewPromptIdentity> {
        self.prompt.as_ref()
    }

    #[must_use]
    pub fn query(&self) -> &PreviewConditionQuery {
        &self.query
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewConditionQuery {
    pub(crate) function: String,
    pub(crate) arguments: Vec<PreviewConditionArgument>,
    pub(crate) expected_type: crate::ConditionExpectedType,
}

impl PreviewConditionQuery {
    #[must_use]
    pub fn function(&self) -> &str {
        &self.function
    }

    #[must_use]
    pub fn arguments(&self) -> &[PreviewConditionArgument] {
        &self.arguments
    }

    #[must_use]
    pub fn expected_type(&self) -> crate::ConditionExpectedType {
        self.expected_type
    }

    pub(crate) fn from_query(query: crate::ConditionQuery<'_>) -> Self {
        Self {
            function: query.function().to_owned(),
            arguments: query
                .arguments()
                .iter()
                .map(PreviewConditionArgument::from)
                .collect(),
            expected_type: query.expected_type(),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewConditionArgument {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<ConditionArgument<'_>> for PreviewConditionArgument {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value.to_owned()),
            ConditionArgument::String(value) => Self::String(value.to_owned()),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewConditionResult {
    Value(ConditionValue),
    Failed { reason: String },
}

impl PreviewConditionResult {
    #[must_use]
    pub fn from_answer(answer: &ConditionAnswer) -> Self {
        match answer {
            ConditionAnswer::Value(value) => Self::Value(value.clone()),
            ConditionAnswer::Failed { reason } => Self::Failed {
                reason: reason.clone(),
            },
        }
    }
}
