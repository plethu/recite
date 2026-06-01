use recite_core::{Block, BlockId, SourceMetadata, Statement};

use crate::body::{BodyBoundary, BodyCursor, BodyStep};
use crate::diagnostics::{empty_block_id, missing_block_id, statement_before_block};
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, span_for_line};

use super::{BlockHeader, Lowerer, header_fields};

impl Lowerer<'_, '_> {
    pub(super) fn lower_blocks(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < self.lines.len() {
            let line = self.lines[index];
            let trimmed = line.trimmed_content();

            if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Block) {
                self.report_statement_before_block(line, trimmed);
                index += 1;
                continue;
            }

            let (block, next_index) = self.lower_block(index);
            if let Some(block) = block {
                blocks.push(block);
            }
            index = next_index;
        }

        blocks
    }

    fn lower_block(&mut self, header_index: usize) -> (Option<Block>, usize) {
        let Some(header) = self.lower_block_header(header_index) else {
            return (None, header_index + 1);
        };

        let (statements, next_index) = self.lower_block_statements(header_index);
        let mut block = Block::new(header.id, statements, header.span)
            .with_default(header.is_default)
            .with_metadata(header.metadata);
        if let Some(default_speaker) = header.default_speaker {
            block = block.with_default_speaker(default_speaker);
        }

        (Some(block), next_index)
    }

    fn lower_block_header(&mut self, header_index: usize) -> Option<BlockHeader> {
        let line = self.lines[header_index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let span = span_for_line(self.path, line.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Block, line, base_column);
        let Some(id_field) = fields.first().copied() else {
            self.diagnostics.push(missing_block_id(span));
            return None;
        };

        if id_field.key_value(self.path).is_some() {
            self.diagnostics
                .push(missing_block_id(id_field.span(self.path)));
            return None;
        }

        let Ok(id) = BlockId::new(id_field.text) else {
            self.diagnostics
                .push(empty_block_id(id_field.span(self.path)));
            return None;
        };

        let mut is_default = false;
        let mut default_speaker = None;
        let mut metadata = SourceMetadata::new();

        for field in fields.iter().skip(1).copied() {
            if field.text == "default" {
                is_default = true;
                continue;
            }

            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "speaker" {
                default_speaker = self.speaker_from_value(&kv);
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        Some(BlockHeader {
            id,
            is_default,
            default_speaker,
            metadata,
            span,
        })
    }

    fn lower_block_statements(&mut self, header_index: usize) -> (Vec<Statement>, usize) {
        let mut statements = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::NextBlock);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            let (statement, next_index) = self.lower_statement(index);
            if let Some(statement) = statement {
                statements.push(statement);
            }
            cursor.set_index(next_index);
        }

        (statements, cursor.index())
    }

    fn report_statement_before_block(&mut self, line: LogicalLine<'_>, trimmed: &str) {
        if trimmed.is_empty()
            || classify_line(line) == ClassifiedLine::Statement(StatementMarker::Comment)
        {
            return;
        }

        self.diagnostics.push(statement_before_block(span_for_line(
            self.path,
            line.number,
            1,
        )));
    }
}
