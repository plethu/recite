use recite_core::{
    Block, BlockId, Comment, Diagnostic, Line, Metadata, MetadataEntry, ScalarValue, SourceFile,
    SourceSpan, SourceText, SpeakerId, Statement,
};

use crate::diagnostics::{
    empty_block_id, missing_block_id, missing_line_id, nested_unsupported_lowering,
    statement_before_block, unsupported_lowering,
};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, LogicalLines, span_for_line};

#[derive(Clone, Debug, PartialEq)]
pub struct LoweredSourceFile {
    pub source_file: SourceFile,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn lower_source_file(
    path: &str,
    source: &str,
    parse_diagnostics: &[Diagnostic],
) -> LoweredSourceFile {
    let mut diagnostics = parse_diagnostics.to_vec();
    let blocks = lower_blocks(path, source, &mut diagnostics);

    LoweredSourceFile {
        source_file: SourceFile::new(path, blocks),
        diagnostics,
    }
}

fn lower_blocks(path: &str, source: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<Block> {
    Lowerer::new(path, source, diagnostics).lower_blocks()
}

struct Lowerer<'source, 'diagnostics> {
    path: &'source str,
    lines: Vec<LogicalLine<'source>>,
    diagnostics: &'diagnostics mut Vec<Diagnostic>,
}

struct LoweredLineText {
    text: String,
    span: SourceSpan,
    next_index: usize,
}

impl<'source, 'diagnostics> Lowerer<'source, 'diagnostics> {
    fn new(
        path: &'source str,
        source: &'source str,
        diagnostics: &'diagnostics mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            path,
            lines: LogicalLines::new(source).collect(),
            diagnostics,
        }
    }

    fn lower_blocks(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < self.lines.len() {
            let line = self.lines[index];
            let trimmed = line.trimmed_content();

            if StatementMarker::parse(trimmed) != Some(StatementMarker::Block) {
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
        let line = self.lines[header_index];
        let trimmed = line.trimmed_content();

        let Some(block_id) = first_field_after_prefix(trimmed, StatementMarker::Block.text())
        else {
            self.diagnostics
                .push(missing_block_id(span_for_line(self.path, line.number, 1)));
            return (None, header_index + 1);
        };

        let block_start = span_for_line(self.path, line.number, 1);
        let is_default = trimmed
            .split_ascii_whitespace()
            .skip(2)
            .any(|field| field == "default");
        let (statements, next_index) = self.lower_block_statements(header_index + 1);

        let Ok(id) = BlockId::new(block_id) else {
            self.diagnostics.push(empty_block_id(block_start.clone()));
            return (None, next_index);
        };

        (
            Some(Block::new(id, statements, block_start).with_default(is_default)),
            next_index,
        )
    }

    fn lower_block_statements(&mut self, mut index: usize) -> (Vec<Statement>, usize) {
        let mut statements = Vec::new();

        while index < self.lines.len() {
            let trimmed = self.lines[index].trimmed_content();

            if StatementMarker::parse(trimmed) == Some(StatementMarker::Block) {
                break;
            }

            let (statement, next_index) = self.lower_block_statement(index, trimmed);
            if let Some(statement) = statement {
                statements.push(statement);
            }
            index = next_index;
        }

        (statements, index)
    }

    fn lower_block_statement(&mut self, index: usize, trimmed: &str) -> (Option<Statement>, usize) {
        match StatementMarker::parse(trimmed) {
            Some(StatementMarker::Line) => {
                let (line, next_index) = self.lower_line(index);
                (Some(Statement::Line(line)), next_index)
            }
            Some(StatementMarker::Comment) => (
                Some(Statement::Comment(self.lower_comment(self.lines[index]))),
                index + 1,
            ),
            Some(marker) if marker.is_unsupported_lowering() => {
                self.report_unsupported_statement(index);
                (None, skip_statement_body(&self.lines, index))
            }
            _ => (None, index + 1),
        }
    }

    fn lower_line(&mut self, line_index: usize) -> (Line, usize) {
        let header = self.lines[line_index];
        let header_indent = header.indent_len();
        let trimmed = header.trimmed_content();
        let line_span = span_for_line(self.path, header.number, header_indent + 1);
        let mut header_fields = header_fields_after_prefix(
            trimmed,
            StatementMarker::Line.text(),
            header.number,
            header_indent + 1,
        );
        let line_id = header_fields.next().map(|field| field.text);
        let (speaker, metadata) = lower_line_header_fields(self.path, header_fields);
        let lowered_text = self.lower_line_text(line_index + 1, header);

        if line_id.is_none() {
            self.diagnostics.push(missing_line_id(line_span.clone()));
        }

        let id = line_id.and_then(|value| recite_core::LineId::new(value).ok());
        let mut line = Line::new(
            id,
            SourceText::new(lowered_text.text, lowered_text.span),
            line_span,
        )
        .with_metadata(metadata);
        if let Some(speaker) = speaker {
            line = line.with_speaker(speaker);
        }

        (line, lowered_text.next_index)
    }

    fn lower_line_text(&mut self, start_index: usize, header: LogicalLine<'_>) -> LoweredLineText {
        let header_indent = header.indent_len();
        let mut text_lines = Vec::new();
        let mut text_start_line = header.number;
        let mut text_start_column = header_indent + 3;
        let mut index = start_index;

        while index < self.lines.len() {
            let current = self.lines[index];
            let trimmed = current.trimmed_content();
            let indent = current.indent_len();

            if StatementMarker::parse(trimmed) == Some(StatementMarker::Block)
                || (indent <= header_indent && is_statement_header(trimmed))
            {
                break;
            }

            if trimmed.is_empty() {
                text_lines.push(String::new());
                index += 1;
                continue;
            }

            if is_statement_header(trimmed) {
                self.report_nested_unsupported_statement(current, indent);
                index = skip_statement_body(&self.lines, index);
                continue;
            }

            if text_lines.is_empty() {
                text_start_line = current.number;
                text_start_column = indent + 1;
            }
            text_lines.push(trimmed.to_owned());
            index += 1;
        }

        LoweredLineText {
            text: text_lines.join("\n"),
            span: span_for_line(self.path, text_start_line, text_start_column),
            next_index: index,
        }
    }

    fn lower_comment(&self, line: LogicalLine<'_>) -> Comment {
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let text = trimmed
            .strip_prefix(StatementMarker::Comment.text())
            .expect("comment lowering only receives comment lines")
            .trim_start_matches([' ', '\t']);

        Comment::new(text, span_for_line(self.path, line.number, indent + 1))
    }

    fn report_statement_before_block(&mut self, line: LogicalLine<'_>, trimmed: &str) {
        if trimmed.is_empty() || StatementMarker::parse(trimmed) == Some(StatementMarker::Comment) {
            return;
        }

        self.diagnostics.push(statement_before_block(span_for_line(
            self.path,
            line.number,
            1,
        )));
    }

    fn report_unsupported_statement(&mut self, line_index: usize) {
        let line = self.lines[line_index];
        self.diagnostics.push(unsupported_lowering(span_for_line(
            self.path,
            line.number,
            1,
        )));
    }

    fn report_nested_unsupported_statement(&mut self, line: LogicalLine<'_>, indent: usize) {
        self.diagnostics
            .push(nested_unsupported_lowering(span_for_line(
                self.path,
                line.number,
                indent + 1,
            )));
    }
}

fn first_field_after_prefix<'a>(trimmed: &'a str, prefix: &str) -> Option<&'a str> {
    trimmed[prefix.len()..].split_ascii_whitespace().next()
}

fn lower_line_header_fields<'a>(
    path: &str,
    fields: impl IntoIterator<Item = HeaderField<'a>>,
) -> (Option<SpeakerId>, Metadata) {
    let mut speaker = None;
    let mut metadata = Metadata::new();

    for field in fields {
        let Some((key, value)) = field.text.split_once('=') else {
            metadata.push(
                MetadataEntry::new(field.text, ScalarValue::Boolean(true))
                    .with_source_span(span_for_line(path, field.line, field.column)),
            );
            continue;
        };

        if key == "speaker" {
            speaker = SpeakerId::new(value).ok();
            continue;
        }

        metadata.push(
            MetadataEntry::new(key, parse_scalar_value(value)).with_source_span(span_for_line(
                path,
                field.line,
                field.column,
            )),
        );
    }

    (speaker, metadata)
}

fn parse_scalar_value(value: &str) -> ScalarValue {
    if value == "true" {
        return ScalarValue::Boolean(true);
    }

    if value == "false" {
        return ScalarValue::Boolean(false);
    }

    if let Ok(integer) = value.parse::<i64>() {
        return ScalarValue::Integer(integer);
    }

    if let Ok(float) = value.parse::<f64>() {
        return ScalarValue::Float(float);
    }

    ScalarValue::String(value.to_owned())
}

fn skip_statement_body(lines: &[LogicalLine<'_>], header_index: usize) -> usize {
    let header_indent = lines[header_index].indent_len();
    let mut index = header_index + 1;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trimmed_content();

        if StatementMarker::parse(trimmed) == Some(StatementMarker::Block) {
            break;
        }

        if !trimmed.is_empty() && line.indent_len() <= header_indent {
            break;
        }

        index += 1;
    }

    index
}

#[derive(Clone, Copy, Debug)]
struct HeaderField<'a> {
    text: &'a str,
    line: u32,
    column: usize,
}

fn header_fields_after_prefix<'a>(
    trimmed: &'a str,
    prefix: &str,
    line: u32,
    base_column: usize,
) -> impl Iterator<Item = HeaderField<'a>> {
    let mut cursor = prefix.len();

    std::iter::from_fn(move || {
        while cursor < trimmed.len() && matches!(trimmed.as_bytes()[cursor], b' ' | b'\t') {
            cursor += 1;
        }

        if cursor >= trimmed.len() {
            return None;
        }

        let start = cursor;
        while cursor < trimmed.len() && !matches!(trimmed.as_bytes()[cursor], b' ' | b'\t') {
            cursor += 1;
        }

        Some(HeaderField {
            text: &trimmed[start..cursor],
            line,
            column: base_column + start,
        })
    })
}

fn is_statement_header(trimmed: &str) -> bool {
    StatementMarker::parse(trimmed).is_some()
}
