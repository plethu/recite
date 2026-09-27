use super::{RuleArgument, replace, values};
use crate::{EditError, projection::offset};
use recite_core::{SourceSpan, ast::EffectMode};
use std::ops::Range;
#[derive(Clone, Debug, PartialEq)]
pub struct RuleEffect {
    pub function: String,
    pub arguments: Vec<RuleArgument>,
    pub mode: EffectMode,
    pub allowed_modes: Vec<EffectMode>,
    pub(super) original: Option<recite_core::ast::Effect>,
    pub(super) raw: String,
}
impl RuleEffect {
    pub(super) fn source(&self) -> Result<String, EditError> {
        let Some(original) = &self.original else {
            if !self.allowed_modes.contains(&self.mode) {
                return Err(EditError::Position);
            }
            let mode = match self.mode {
                EffectMode::Deferred => "deferred",
                EffectMode::Immediate => "immediate",
                EffectMode::Blocking => "blocking",
            };
            return Ok(format!(
                "! {mode} {}({})",
                self.function,
                values::arguments_source(&self.arguments)?
            ));
        };
        let mut patches = Vec::new();
        let base = original.span.start.column();
        let relative = |span: &SourceSpan| -> Result<Range<usize>, EditError> {
            let span_end = span.end.ok_or(EditError::Position)?;
            if span.start.line() != original.span.start.line()
                || span_end.line() != span.start.line()
            {
                return Err(EditError::SourceRequired("multiline effect arguments"));
            }
            let start = recite_core::SourcePosition::new(1, span.start.column() - base + 1)?;
            let end = recite_core::SourcePosition::new(1, span_end.column() - base + 2)?;
            Ok(offset(&self.raw, start)?..offset(&self.raw, end)?)
        };
        if self.mode != original.mode {
            if !self.allowed_modes.contains(&self.mode) {
                return Err(EditError::SourceRequired(
                    "effect delivery is not supported by its declaration",
                ));
            }
            let mode = match self.mode {
                EffectMode::Deferred => "deferred",
                EffectMode::Immediate => "immediate",
                EffectMode::Blocking => "blocking",
            };
            patches.push((
                relative(original.mode_span.as_ref().ok_or(EditError::Position)?)?,
                mode.into(),
            ));
        }
        for (argument, span) in self.arguments.iter().zip(&original.arg_spans) {
            if argument.changed() {
                patches.push((relative(span)?, argument.source()?));
            }
        }
        replace(&self.raw, patches)
    }
}
