use crate::SourceSpan;

use super::Argument;

/// A standalone effect request statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub mode: EffectMode,
    pub function: String,
    pub args: Vec<Argument>,
    pub span: SourceSpan,
}

impl Effect {
    #[must_use]
    pub fn new(
        mode: EffectMode,
        function: impl Into<String>,
        args: Vec<Argument>,
        span: SourceSpan,
    ) -> Self {
        Self {
            mode,
            function: function.into(),
            args,
            span,
        }
    }
}

/// Effect emission mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EffectMode {
    Deferred,
    Immediate,
    Blocking,
}
