use super::{RuleArgument, values};
use crate::EditError;
#[derive(Clone, Debug, PartialEq)]
pub enum RuleExpression {
    Call {
        function: String,
        arguments: Vec<RuleArgument>,
    },
    All(Vec<RuleExpression>),
    Any(Vec<RuleExpression>),
    Not(Box<RuleExpression>),
    Group(Box<RuleExpression>),
}
impl RuleExpression {
    pub(super) fn validate(&self) -> Result<(), EditError> {
        match self {
            Self::Call { arguments, .. } => arguments.iter().try_for_each(RuleArgument::validate),
            Self::All(items) | Self::Any(items) => items.iter().try_for_each(Self::validate),
            Self::Not(inner) | Self::Group(inner) => inner.validate(),
        }
    }

    pub(super) fn source(&self) -> Result<String, EditError> {
        Ok(match self {
            Self::Call {
                function,
                arguments,
            } => format!("{function}({})", values::arguments_source(arguments)?),
            Self::All(items) | Self::Any(items) => {
                let join = if matches!(self, Self::All(_)) {
                    " and "
                } else {
                    " or "
                };
                let parts = items
                    .iter()
                    .map(Self::source)
                    .collect::<Result<Vec<_>, _>>()?;
                format!("({})", parts.join(join))
            }
            Self::Not(inner) => format!("not ({})", inner.source()?),
            Self::Group(inner) => format!("({})", inner.source()?),
        })
    }
}
