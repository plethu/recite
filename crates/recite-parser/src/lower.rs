mod block;
mod body;
mod branch;
mod metadata;
mod metadata_values;
mod statement;

use recite_core::{
    BlockId, Diagnostic, SourceFile, SourceMetadata, SourceSpan, SpeakerId, Statement,
};

use crate::header::{HeaderField, fields_after_prefix};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, LogicalLines};

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
    let blocks = Lowerer::new(path, source, &mut diagnostics).lower_blocks();

    LoweredSourceFile {
        source_file: SourceFile::new(path, blocks),
        diagnostics,
    }
}

pub(super) struct Lowerer<'source, 'diagnostics> {
    path: &'source str,
    lines: Vec<LogicalLine<'source>>,
    diagnostics: &'diagnostics mut Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(super) struct LoweredProseBody {
    pub(super) text: String,
    pub(super) text_span: SourceSpan,
    pub(super) plural_text: Option<String>,
    pub(super) plural_text_span: Option<SourceSpan>,
    pub(super) statements: Vec<Statement>,
    pub(super) next_index: usize,
}

#[derive(Clone, Debug)]
pub(super) struct BlockHeader {
    pub(super) id: BlockId,
    pub(super) id_span: SourceSpan,
    pub(super) is_default: bool,
    pub(super) default_speaker: Option<SpeakerId>,
    pub(super) metadata: SourceMetadata,
    pub(super) span: SourceSpan,
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
}

pub(super) fn header_fields<'a>(
    trimmed: &'a str,
    marker: StatementMarker,
    line: LogicalLine<'_>,
    base_column: usize,
) -> Vec<HeaderField<'a>> {
    fields_after_prefix(trimmed, marker.text(), line.number, base_column).collect()
}

pub(super) fn insertion_span_after_marker(
    path: &str,
    line: LogicalLine<'_>,
    marker: StatementMarker,
) -> SourceSpan {
    let trimmed = line.trimmed_content();
    let rest = &trimmed[marker.text().len()..];
    let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    crate::source::span_for_line(
        path,
        line.number,
        line.indent_len() + 1 + marker.text().chars().count() + whitespace_len,
    )
}
