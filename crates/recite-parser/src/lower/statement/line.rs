use recite_core::{Line, LineId, SourceText};

use crate::markers::StatementMarker;
use crate::source::span_for_line;

use super::super::{Lowerer, header_fields};

impl Lowerer<'_, '_> {
    pub(super) fn lower_line(&mut self, line_index: usize) -> (Line, usize) {
        let header = self.lines[line_index];
        let indent = header.indent_len();
        let trimmed = header.trimmed_content();
        let base_column = indent + 1;
        let line_span = span_for_line(self.path, header.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Line, header, base_column);
        let mut field_start = 0;
        let line_id = if let Some(first) = fields.first().copied() {
            if first.key_value(self.path).is_none() {
                field_start = 1;
                LineId::new(first.text).ok()
            } else {
                None
            }
        } else {
            None
        };

        let (speaker, metadata) = self.lower_speaker_metadata(&fields[field_start..]);
        let body = self.lower_prose_body(line_index, false);
        let mut line = Line::new(
            line_id,
            SourceText::new(body.text, body.text_span),
            line_span,
        )
        .with_metadata(metadata)
        .with_statements(body.statements);
        if let Some(speaker) = speaker {
            line = line.with_speaker(speaker);
        }

        (line, body.next_index)
    }
}
