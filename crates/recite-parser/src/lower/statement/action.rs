use recite_core::{Divert, Effect};

use crate::condition::parse_condition_call;
use crate::diagnostics::{malformed_divert_target, malformed_effect, missing_divert_target};
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
            self.diagnostics
                .push(malformed_effect(span, "missing effect mode"));
            return None;
        };
        let Some(mode) = effect_mode(mode_field.text) else {
            self.diagnostics.push(malformed_effect(
                mode_field.span(self.path),
                "expected deferred, immediate, or blocking",
            ));
            return None;
        };

        let call_start = mode_field.offset + mode_field.text.len();
        let rest = &trimmed[call_start..];
        let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let call_text = &rest[whitespace_len..];
        let call_column = base_column + trimmed[..call_start + whitespace_len].chars().count();

        match parse_condition_call(self.path, line.number, call_column, call_text) {
            Ok(call) => Some(Effect::new(mode, call.function, call.args, span)),
            Err(error) => {
                self.diagnostics
                    .push(malformed_effect(error.span, error.message));
                None
            }
        }
    }
}
