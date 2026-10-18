use recite_core::{
    BlockId, SourceRecoveryClass,
    ast::{Block, SourceMetadata, Statement},
};

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
        while let Some((header_index, header)) = self.next_block_header(&mut index) {
            let mut statements = Vec::new();
            let result = self.visit_block_statements(header_index, |statement, sole| {
                if sole {
                    statements.reserve_exact(1);
                }
                statements.push(statement);
                Ok::<(), std::convert::Infallible>(())
            });
            index = match result {
                Ok(index) => index,
                Err(impossible) => match impossible {},
            };
            blocks.push(header.into_block(statements));
        }
        blocks
    }

    pub(super) fn visit_roots<E>(
        &mut self,
        mut visit: impl FnMut(&Block, &Statement) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut index = 0;
        while let Some((header_index, header)) = self.next_block_header(&mut index) {
            let block = header.into_block(Vec::new());
            index = self
                .visit_block_statements(header_index, |statement, _| visit(&block, &statement))?;
        }
        Ok(())
    }

    fn next_block_header(&mut self, index: &mut usize) -> Option<(usize, BlockHeader)> {
        while *index < self.lines.len() {
            let header_index = *index;
            let line = self.lines[header_index];
            *index += 1;
            if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Block) {
                self.report_statement_before_block(line, line.trimmed_content());
            } else if let Some(header) = self.lower_block_header(header_index) {
                return Some((header_index, header));
            }
        }
        None
    }

    fn lower_block_header(&mut self, header_index: usize) -> Option<BlockHeader> {
        let line = self.lines[header_index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let span = span_for_line(self.path, line.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Block, line, base_column);
        let Some(id_field) = fields.first().copied() else {
            self.mark(SourceRecoveryClass::BlockDefinitions);
            self.mark(SourceRecoveryClass::AstStructure);
            self.diagnostics.push(missing_block_id(span));
            return None;
        };

        if id_field.key_value(self.path).is_some() {
            self.mark(SourceRecoveryClass::BlockDefinitions);
            self.mark(SourceRecoveryClass::AstStructure);
            self.diagnostics
                .push(missing_block_id(id_field.span(self.path)));
            return None;
        }

        let Ok(id) = BlockId::new(id_field.text) else {
            self.mark(SourceRecoveryClass::BlockDefinitions);
            self.mark(SourceRecoveryClass::AstStructure);
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
            id_span: id_field.span(self.path),
            is_default,
            default_speaker,
            metadata,
            span,
        })
    }

    fn visit_block_statements<E>(
        &mut self,
        header_index: usize,
        mut visit: impl FnMut(Statement, bool) -> Result<(), E>,
    ) -> Result<usize, E> {
        let mut emitted = false;
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::NextBlock);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank => continue,
                BodyStep::MixedIndent => {
                    super::mark_all(self.recovery);
                    continue;
                }
            };

            let (statement, next_index) = self.lower_statement(index);
            if let Some(statement) = statement {
                let sole = !emitted
                    && self.lines[next_index..]
                        .iter()
                        .find(|line| !line.trimmed_content().is_empty())
                        .is_none_or(|line| cursor.is_boundary(*line));
                visit(statement, sole)?;
                emitted = true;
            }
            cursor.set_index(next_index);
        }

        Ok(cursor.index())
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
        super::mark_all(self.recovery);
    }
}

impl BlockHeader {
    fn into_block(self, statements: Vec<Statement>) -> Block {
        let mut block = Block::new(self.id, statements, self.span)
            .with_id_span(self.id_span)
            .with_default(self.is_default)
            .with_metadata(self.metadata);
        if let Some(speaker) = self.default_speaker {
            block = block.with_default_speaker(speaker);
        }
        block
    }
}
