mod block;
mod body;
mod branch;
mod metadata;
mod metadata_values;
mod statement;

use recite_core::{
    BlockId, Diagnostic, SourceId, SourceRecovery, SourceRecoveryClass, SourceSpan, SpeakerId,
    ast::{SourceFile, SourceMetadata, Statement},
};

use crate::header::{HeaderField, fields_after_prefix};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, LogicalLines};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LoweredSourceFile {
    pub source_file: SourceFile,
    pub diagnostics: Vec<Diagnostic>,
    pub recovery: SourceRecovery,
}

pub(crate) fn lower_source_file(
    path: &str,
    source: &str,
    parse_diagnostics: &[Diagnostic],
    parse_recovery: SourceRecovery,
    first_line: u32,
) -> LoweredSourceFile {
    let mut diagnostics = parse_diagnostics.to_vec();
    let mut recovery = parse_recovery;
    let blocks =
        Lowerer::new(path, source, &mut diagnostics, &mut recovery, first_line).lower_blocks();

    LoweredSourceFile {
        source_file: SourceFile::new(path, blocks),
        diagnostics,
        recovery,
    }
}

/// Syntax/lowering diagnostics and recovery after streaming all root statements.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SourceVisitReport {
    pub diagnostics: Vec<Diagnostic>,
    pub recovery: SourceRecovery,
}

pub(crate) fn visit_source_roots<E>(
    path: &str,
    source: &str,
    first_line: u32,
    mut diagnostics: Vec<Diagnostic>,
    mut recovery: SourceRecovery,
    visit: impl FnMut(&recite_core::ast::Block, &Statement) -> Result<(), E>,
) -> Result<SourceVisitReport, E> {
    Lowerer::new(path, source, &mut diagnostics, &mut recovery, first_line).visit_roots(visit)?;
    Ok(SourceVisitReport {
        diagnostics,
        recovery,
    })
}

pub(super) struct Lowerer<'source, 'diagnostics> {
    path: &'source str,
    lines: Vec<LogicalLine<'source>>,
    diagnostics: &'diagnostics mut Vec<Diagnostic>,
    pub(super) recovery: &'diagnostics mut SourceRecovery,
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
        recovery: &'diagnostics mut SourceRecovery,
        first_line: u32,
    ) -> Self {
        Self {
            path,
            lines: LogicalLines::starting_at(source, first_line).collect(),
            diagnostics,
            recovery,
        }
    }

    // A sole statement needs one slot, not Vec's default four large AST slots.
    // Bodies that may contain more statements keep normal growth.
    pub(super) fn reserve_single_statement(
        &self,
        statements: &mut Vec<Statement>,
        cursor: &crate::body::BodyCursor,
        next_index: usize,
    ) {
        if statements.capacity() == 0
            && self.lines[next_index..]
                .iter()
                .find(|line| !line.trimmed_content().is_empty())
                .is_none_or(|line| cursor.is_boundary(*line))
        {
            statements.reserve_exact(1);
        }
    }

    pub(super) fn mark(&mut self, class: SourceRecoveryClass) {
        self.recovery.mark(class);
    }
}

pub(super) fn mark_all(recovery: &mut SourceRecovery) {
    for class in [
        SourceRecoveryClass::AstStructure,
        SourceRecoveryClass::BlockDefinitions,
        SourceRecoveryClass::BlockReferences,
        SourceRecoveryClass::StableIds,
        SourceRecoveryClass::Metadata,
        SourceRecoveryClass::ConditionFunctions,
        SourceRecoveryClass::EffectFunctions,
        SourceRecoveryClass::InlineMarkup,
    ] {
        recovery.mark(class);
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

/// The trailing stable-ID field of a line or choice header.
pub(super) struct HeaderSourceId {
    pub(super) source_id: SourceId,
    pub(super) span: Option<SourceSpan>,
    /// Point after the last non-ID field, where new fields or a missing
    /// ` @id` are inserted.
    pub(super) fields_end: SourceSpan,
}

/// Splits the stable ID from the other header fields.
///
/// Only the last field may be an `@id`. An `@` field anywhere else, or more than
/// one, is malformed so that every header spells its identity in one place.
pub(super) fn split_source_id<'a>(
    path: &str,
    line: LogicalLine<'_>,
    marker: StatementMarker,
    fields: &[HeaderField<'a>],
) -> (HeaderSourceId, Vec<HeaderField<'a>>) {
    let rest = fields
        .iter()
        .copied()
        .filter(|field| !SourceId::is_field(field.text))
        .collect::<Vec<_>>();
    let fields_end = rest.last().map_or_else(
        || insertion_span_after_marker(path, line, marker),
        |last| {
            crate::source::span_for_line(path, line.number, last.column + last.text.chars().count())
        },
    );
    let mut id_fields = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| SourceId::is_field(field.text));
    let (source_id, span) = match (id_fields.next(), id_fields.next()) {
        (None, _) => (SourceId::Missing, None),
        (Some((index, field)), None) if index + 1 == fields.len() => {
            (SourceId::parse_field(field.text), Some(field.span(path)))
        }
        (Some((_, field)), _) => (
            SourceId::Malformed {
                raw: field.text.to_owned(),
            },
            Some(field.span(path)),
        ),
    };
    (
        HeaderSourceId {
            source_id,
            span,
            fields_end,
        },
        rest,
    )
}

fn insertion_span_after_marker(
    path: &str,
    line: LogicalLine<'_>,
    marker: StatementMarker,
) -> SourceSpan {
    crate::source::span_for_line(
        path,
        line.number,
        line.indent_len() + 1 + marker.text().chars().count(),
    )
}
