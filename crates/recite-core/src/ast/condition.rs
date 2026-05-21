use crate::{ScalarValue, SourceSpan};

/// Boolean condition expression tree.
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionExpression {
    Call(ConditionCall),
    And(ConditionGroup),
    Or(ConditionGroup),
    Not(ConditionUnary),
    Grouped(ConditionUnary),
}

impl ConditionExpression {
    #[must_use]
    pub fn call(function: impl Into<String>, args: Vec<Argument>, span: SourceSpan) -> Self {
        Self::Call(ConditionCall::new(function, args, span))
    }

    #[must_use]
    pub fn and(expressions: Vec<ConditionExpression>, span: SourceSpan) -> Self {
        Self::And(ConditionGroup::new(expressions, span))
    }

    #[must_use]
    pub fn or(expressions: Vec<ConditionExpression>, span: SourceSpan) -> Self {
        Self::Or(ConditionGroup::new(expressions, span))
    }

    #[must_use]
    pub fn not(expression: ConditionExpression, span: SourceSpan) -> Self {
        Self::Not(ConditionUnary::new(expression, span))
    }

    #[must_use]
    pub fn grouped(expression: ConditionExpression, span: SourceSpan) -> Self {
        Self::Grouped(ConditionUnary::new(expression, span))
    }

    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Call(call) => &call.span,
            Self::And(group) | Self::Or(group) => &group.span,
            Self::Not(unary) | Self::Grouped(unary) => &unary.span,
        }
    }
}

/// A source-spanned variadic condition expression group.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionGroup {
    pub expressions: Vec<ConditionExpression>,
    pub span: SourceSpan,
}

impl ConditionGroup {
    #[must_use]
    pub fn new(expressions: Vec<ConditionExpression>, span: SourceSpan) -> Self {
        Self { expressions, span }
    }
}

/// A source-spanned unary condition expression.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionUnary {
    pub expression: Box<ConditionExpression>,
    pub span: SourceSpan,
}

impl ConditionUnary {
    #[must_use]
    pub fn new(expression: ConditionExpression, span: SourceSpan) -> Self {
        Self {
            expression: Box::new(expression),
            span,
        }
    }
}

/// A condition-language function call.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionCall {
    pub function: String,
    pub function_span: Option<SourceSpan>,
    pub args: Vec<Argument>,
    pub arg_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

impl ConditionCall {
    #[must_use]
    pub fn new(function: impl Into<String>, args: Vec<Argument>, span: SourceSpan) -> Self {
        Self {
            function: function.into(),
            function_span: None,
            args,
            arg_spans: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_source_spans(
        mut self,
        function_span: SourceSpan,
        arg_spans: Vec<SourceSpan>,
    ) -> Self {
        self.function_span = Some(function_span);
        self.arg_spans = arg_spans;
        self
    }
}

/// A typed source argument. Bare identifiers are distinct from string literals.
#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    Identifier(String),
    Value(ScalarValue),
}

impl Argument {
    #[must_use]
    pub fn identifier(value: impl Into<String>) -> Self {
        Self::Identifier(value.into())
    }
}

impl From<ScalarValue> for Argument {
    fn from(value: ScalarValue) -> Self {
        Self::Value(value)
    }
}
