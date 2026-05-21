use crate::SourceSpan;

use super::Argument;

/// A standalone effect request statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub mode: EffectMode,
    pub mode_span: Option<SourceSpan>,
    pub function: String,
    pub function_span: Option<SourceSpan>,
    pub args: Vec<Argument>,
    pub arg_spans: Vec<SourceSpan>,
    pub call_span: Option<SourceSpan>,
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
            mode_span: None,
            function: function.into(),
            function_span: None,
            args,
            arg_spans: Vec::new(),
            call_span: None,
            span,
        }
    }

    #[must_use]
    pub fn with_source_spans(
        mut self,
        mode_span: SourceSpan,
        call_span: SourceSpan,
        function_span: SourceSpan,
        arg_spans: Vec<SourceSpan>,
    ) -> Self {
        self.mode_span = Some(mode_span);
        self.call_span = Some(call_span);
        self.function_span = Some(function_span);
        self.arg_spans = arg_spans;
        self
    }
}

/// Effect emission mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum EffectMode {
    Deferred,
    Immediate,
    Blocking,
}
