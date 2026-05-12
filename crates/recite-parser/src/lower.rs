use recite_core::{
    Block, BlockId, Comment, Diagnostic, Line, Metadata, MetadataEntry, ScalarValue, SourceFile,
    SourceText, SpeakerId, Statement,
};

use crate::diagnostics::diagnostic;
use crate::source::{LogicalLine, LogicalLines, indent_len, span_for_line};

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
    let mut blocks = Vec::new();
    let lines: Vec<_> = LogicalLines::new(source).collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line
            .content_without_newline()
            .trim_start_matches([' ', '\t']);

        if !trimmed.starts_with("::") {
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                diagnostics.push(diagnostic(
                    "RECITE_PARSE002",
                    "statement appears before a block header",
                    span_for_line(path, line.number, 1),
                ));
            }
            index += 1;
            continue;
        }

        let Some(block_id) = first_field_after_prefix(trimmed, "::") else {
            diagnostics.push(diagnostic(
                "RECITE_PARSE003",
                "block header must include a block id",
                span_for_line(path, line.number, 1),
            ));
            index += 1;
            continue;
        };

        let mut statements = Vec::new();
        let block_start = span_for_line(path, line.number, 1);
        let is_default = trimmed
            .split_ascii_whitespace()
            .skip(2)
            .any(|field| field == "default");

        index += 1;
        while index < lines.len() {
            let statement_line = lines[index];
            let content = statement_line.content_without_newline();
            let trimmed_statement = content.trim_start_matches([' ', '\t']);

            if trimmed_statement.starts_with("::") {
                break;
            }

            if trimmed_statement.starts_with('>') {
                let (line, next_index) = lower_line(path, &lines, index, diagnostics);
                statements.push(Statement::Line(line));
                index = next_index;
                continue;
            }

            if trimmed_statement.starts_with('#') {
                statements.push(Statement::Comment(lower_comment(path, statement_line)));
                index += 1;
                continue;
            }

            if is_unsupported_statement_header(trimmed_statement) {
                diagnostics.push(diagnostic(
                    "RECITE_PARSE004",
                    "statement syntax is parsed losslessly but lowering is not implemented yet",
                    span_for_line(path, statement_line.number, 1),
                ));
                index = skip_statement_body(&lines, index);
                continue;
            }

            index += 1;
        }

        let Ok(id) = BlockId::new(block_id) else {
            diagnostics.push(diagnostic(
                "RECITE_PARSE005",
                "block id must not be empty",
                block_start.clone(),
            ));
            continue;
        };

        blocks.push(Block::new(id, statements, block_start).with_default(is_default));
    }

    blocks
}

fn lower_line(
    path: &str,
    lines: &[LogicalLine<'_>],
    line_index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Line, usize) {
    let header = lines[line_index];
    let header_content = header.content_without_newline();
    let header_indent = indent_len(header_content);
    let trimmed = header_content.trim_start_matches([' ', '\t']);
    let line_span = span_for_line(path, header.number, header_indent + 1);
    let mut header_fields =
        header_fields_after_prefix(trimmed, ">", header.number, header_indent + 1);
    let line_id = header_fields.next().map(|field| field.text);
    let (speaker, metadata) = lower_line_header_fields(path, header_fields);
    let mut text_lines = Vec::new();
    let mut text_start_line = header.number;
    let mut text_start_column = header_indent + 3;
    let mut index = line_index + 1;

    while index < lines.len() {
        let current = lines[index];
        let current_content = current.content_without_newline();
        let current_trimmed = current_content.trim_start_matches([' ', '\t']);

        if current_trimmed.starts_with("::")
            || (indent_len(current_content) <= header_indent
                && is_statement_header(current_trimmed))
        {
            break;
        }

        if current_trimmed.is_empty() {
            text_lines.push(String::new());
            index += 1;
            continue;
        }

        if is_statement_header(current_trimmed) {
            diagnostics.push(diagnostic(
                "RECITE_PARSE004",
                "nested statement syntax is parsed losslessly but lowering is not implemented yet",
                span_for_line(path, current.number, indent_len(current_content) + 1),
            ));
            index = skip_statement_body(lines, index);
            continue;
        }

        if text_lines.is_empty() {
            text_start_line = current.number;
            text_start_column = indent_len(current_content) + 1;
        }
        text_lines.push(current_trimmed.to_owned());
        index += 1;
    }

    let id = line_id.and_then(|value| recite_core::LineId::new(value).ok());
    let text = text_lines.join("\n");
    let text_span = span_for_line(path, text_start_line, text_start_column);

    if line_id.is_none() {
        diagnostics.push(diagnostic(
            "RECITE_PARSE006",
            "line header has no line id; semantic validation may require one",
            line_span.clone(),
        ));
    }

    let mut line =
        Line::new(id, SourceText::new(text, text_span), line_span).with_metadata(metadata);
    if let Some(speaker) = speaker {
        line = line.with_speaker(speaker);
    }

    (line, index)
}

fn lower_comment(path: &str, line: LogicalLine<'_>) -> Comment {
    let content = line.content_without_newline();
    let indent = indent_len(content);
    let trimmed = content.trim_start_matches([' ', '\t']);
    let text = trimmed
        .strip_prefix('#')
        .expect("comment lowering only receives comment lines")
        .trim_start_matches([' ', '\t']);

    Comment::new(text, span_for_line(path, line.number, indent + 1))
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
    let header_indent = indent_len(lines[header_index].content_without_newline());
    let mut index = header_index + 1;

    while index < lines.len() {
        let content = lines[index].content_without_newline();
        let trimmed = content.trim_start_matches([' ', '\t']);

        if trimmed.starts_with("::") {
            break;
        }

        if !trimmed.is_empty() && indent_len(content) <= header_indent {
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
    trimmed.starts_with("::")
        || trimmed.starts_with('>')
        || trimmed.starts_with('?')
        || trimmed.starts_with('!')
        || trimmed.starts_with("->")
        || trimmed.starts_with(":if")
        || trimmed.starts_with(":else")
        || trimmed.starts_with(":match")
        || trimmed.starts_with(":case")
        || trimmed.starts_with('#')
}

fn is_unsupported_statement_header(trimmed: &str) -> bool {
    trimmed.starts_with('?')
        || trimmed.starts_with('!')
        || trimmed.starts_with("->")
        || trimmed.starts_with(":if")
        || trimmed.starts_with(":else")
        || trimmed.starts_with(":match")
        || trimmed.starts_with(":case")
}
