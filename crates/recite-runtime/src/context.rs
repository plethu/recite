use recite_core::{CompiledArgument, ScalarValue};

/// Caller-provided pure condition evaluation for runtime traversal.
///
/// Context implementations bridge Recite condition calls to host game state.
/// They should be deterministic queries: do not mutate game state, emit effects,
/// or depend on unordered host data.
///
/// # Example
///
/// ```
/// use recite_runtime::{
///     ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext,
/// };
///
/// struct InventoryContext {
///     has_map: bool,
/// }
///
/// impl DialogueContext for InventoryContext {
///     fn evaluate_condition(
///         &self,
///         query: ConditionQuery<'_>,
///     ) -> Result<ConditionValue, ConditionEvaluationError> {
///         match query.function() {
///             "has_map" => Ok(ConditionValue::Bool(self.has_map)),
///             other => Err(ConditionEvaluationError::new(format!(
///                 "no condition handler registered for `{other}`",
///             ))),
///         }
///     }
/// }
///
/// let context = InventoryContext { has_map: true };
/// let _runtime_context: &dyn DialogueContext = &context;
/// ```
pub trait DialogueContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError>;
}

impl<F> DialogueContext for F
where
    F: for<'a> Fn(ConditionQuery<'a>) -> Result<ConditionValue, ConditionEvaluationError>,
{
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
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
    ) -> Result<ConditionValue, ConditionEvaluationError> {
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
    expected_type: ConditionExpectedType,
}

impl<'a> ConditionQuery<'a> {
    pub(crate) fn new(
        function: &'a str,
        arguments: &'a [CompiledArgument],
        expected_type: ConditionExpectedType,
    ) -> Self {
        Self {
            function,
            arguments: ConditionArguments { arguments },
            expected_type,
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

    #[must_use]
    pub fn expected_type(&self) -> ConditionExpectedType {
        self.expected_type
    }
}

/// Runtime condition result requested from the host context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionValue {
    Bool(bool),
    EnumVariant(String),
}

impl ConditionValue {
    #[must_use]
    pub fn kind(&self) -> ConditionExpectedType {
        match self {
            Self::Bool(_) => ConditionExpectedType::Bool,
            Self::EnumVariant(_) => ConditionExpectedType::Enum,
        }
    }
}

/// Result kind expected by a runtime condition query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionExpectedType {
    Bool,
    Enum,
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
