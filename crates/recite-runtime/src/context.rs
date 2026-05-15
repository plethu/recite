use recite_core::{CompiledArgument, ScalarValue};

/// Caller-provided pure condition evaluation for runtime traversal.
pub trait DialogueContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError>;
}

impl<F> DialogueContext for F
where
    F: for<'a> Fn(ConditionQuery<'a>) -> Result<bool, ConditionEvaluationError>,
{
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError> {
        self(query)
    }
}

/// Empty context for assets that are not expected to evaluate conditions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmptyDialogueContext;

impl DialogueContext for EmptyDialogueContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError> {
        Err(ConditionEvaluationError::new(format!(
            "no condition handler registered for `{}`",
            query.function()
        )))
    }
}

/// Borrowed runtime-facing view of one condition call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionQuery<'a> {
    function: &'a str,
    arguments: ConditionArguments<'a>,
}

impl<'a> ConditionQuery<'a> {
    pub(crate) fn new(function: &'a str, arguments: &'a [CompiledArgument]) -> Self {
        Self {
            function,
            arguments: ConditionArguments { arguments },
        }
    }

    #[must_use]
    pub fn function(&self) -> &'a str {
        self.function
    }

    #[must_use]
    pub fn arguments(&self) -> ConditionArguments<'a> {
        self.arguments
    }
}

/// Borrowed condition arguments that preserve identifiers separately from strings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionArguments<'a> {
    arguments: &'a [CompiledArgument],
}

impl<'a> ConditionArguments<'a> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.arguments.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arguments.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = ConditionArgument<'a>> {
        self.arguments.iter().map(ConditionArgument::from)
    }
}

impl<'a> IntoIterator for ConditionArguments<'a> {
    type IntoIter = ConditionArgumentIter<'a>;
    type Item = ConditionArgument<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ConditionArgumentIter {
            arguments: self.arguments.iter(),
        }
    }
}

pub struct ConditionArgumentIter<'a> {
    arguments: std::slice::Iter<'a, CompiledArgument>,
}

impl<'a> Iterator for ConditionArgumentIter<'a> {
    type Item = ConditionArgument<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.arguments.next().map(ConditionArgument::from)
    }
}

/// One borrowed condition argument visible to the host context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConditionArgument<'a> {
    Identifier(&'a str),
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl<'a> From<&'a CompiledArgument> for ConditionArgument<'a> {
    fn from(argument: &'a CompiledArgument) -> Self {
        match argument {
            CompiledArgument::Identifier(value) => Self::Identifier(value),
            CompiledArgument::Value(ScalarValue::String(value)) => Self::String(value),
            CompiledArgument::Value(ScalarValue::Integer(value)) => Self::Integer(*value),
            CompiledArgument::Value(ScalarValue::Float(value)) => Self::Float(*value),
            CompiledArgument::Value(ScalarValue::Boolean(value)) => Self::Boolean(*value),
        }
    }
}

/// Error returned by the caller-provided condition context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionEvaluationError {
    reason: String,
}

impl ConditionEvaluationError {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for ConditionEvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for ConditionEvaluationError {}
