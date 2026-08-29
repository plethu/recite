use recite_core::{Divert, Effect};

use crate::condition::parse_condition_call;
use crate::diagnostics::{
    malformed_divert_target, malformed_effect, malformed_effect_invalid_mode,
    malformed_effect_missing_mode, missing_divert_target,
};
use crate::header::rest_after_field;
use crate::markers::StatementMarker;
use crate::source::span_for_line;

use super::super::metadata::effect_mode;
use super::super::{Lowerer, header_fields};

impl Lowerer<'_, '_> {
    pub(super) fn lower_divert(&mut self, index: usize) -> Option<Divert> {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let fields = header_fields(trimmed, StatementMarker::Divert, line, base_column);
        let span = span_for_line(self.path, line.number, base_column);
        let Some(target) = fields.first().copied() else {
            self.diagnostics.push(missing_divert_target(span));
            return None;
        };

        if fields.len() > 1 {
            self.diagnostics
                .push(malformed_divert_target(fields[1].span(self.path)));
            return None;
        }

        let target = self.divert_target(target)?;

        Some(Divert::new(target, span))
    }

    pub(super) fn lower_effect(&mut self, index: usize) -> Option<Effect> {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let span = span_for_line(self.path, line.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Effect, line, base_column);
        let Some(mode_field) = fields.first().copied() else {
            self.diagnostics.push(malformed_effect_missing_mode(span));
            return None;
        };
        let Some(mode) = effect_mode(mode_field.text) else {
            self.diagnostics
                .push(malformed_effect_invalid_mode(mode_field.span(self.path)));
            return None;
        };

        let call = rest_after_field(trimmed, mode_field);

        match parse_condition_call(self.path, line.number, call.column, call.text) {
            Ok(call) => {
                let function_span = call.function_span.unwrap_or_else(|| {
                    // Parser-created calls carry function spans; fall back to the call span if
                    // that invariant is weakened by a future constructor.
                    call.span.clone()
                });
                Some(
                    Effect::new(mode, call.function, call.args, span).with_source_spans(
                        mode_field.span(self.path),
                        call.span,
                        function_span,
                        call.arg_spans,
                    ),
                )
            }
            Err(error) => {
                self.diagnostics.push(malformed_effect(error));
                None
            }
        }
    }
}
