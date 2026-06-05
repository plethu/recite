mod collector;
mod identity;
mod items;

use std::path::Path;

use lsp_types::Uri;
use recite_core::{Diagnostic, SourcePosition, SourceSpan};
use recite_parser::parse;

use collector::FileSummaryCollector;
pub(crate) use identity::{FileIdentity, OpenFileIdentity, SavedFileIdentity};
pub(crate) use items::{
    BlockReferenceSummary, FileSummaryCompleteness, FunctionReferenceSummary, MetadataKeySummary,
    MissingIdKind, MissingIdSummary, SpannedName,
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct FileSummary {
    pub(crate) identity: FileIdentity,
    pub(crate) version: Option<i32>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) completeness: FileSummaryCompleteness,
    pub(crate) blocks: Vec<SpannedName>,
    pub(crate) block_references: Vec<BlockReferenceSummary>,
    pub(crate) line_ids: Vec<SpannedName>,
    pub(crate) choice_ids: Vec<SpannedName>,
    pub(crate) missing_ids: Vec<MissingIdSummary>,
    pub(crate) metadata_keys: Vec<MetadataKeySummary>,
    pub(crate) condition_functions: Vec<FunctionReferenceSummary>,
    pub(crate) effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummary {
    pub(crate) fn from_text(identity: FileIdentity, version: Option<i32>, text: &str) -> Self {
        let parse = parse(identity.uri().as_str(), text);
        let lowered = parse.lower_source_file();
        let diagnostics = unique_diagnostics(lowered.diagnostics.clone());
        let mut collector = FileSummaryCollector::new();
        collector.collect_source_file(&lowered.source_file);
        narrow_block_reference_spans(&mut collector.block_references, text);
        let complete_source_model = lowered.diagnostics.is_empty();

        Self {
            identity,
            version,
            diagnostics,
            completeness: FileSummaryCompleteness {
                block_definitions: complete_source_model,
                block_references: complete_source_model,
                stable_ids: complete_source_model,
                metadata: complete_source_model,
                condition_functions: complete_source_model,
                effect_functions: complete_source_model,
                inline_markup: false,
                recoverable_regions: false,
            },
            blocks: collector.blocks,
            block_references: collector.block_references,
            line_ids: collector.line_ids,
            choice_ids: collector.choice_ids,
            missing_ids: collector.missing_ids,
            metadata_keys: collector.metadata_keys,
            condition_functions: collector.condition_functions,
            effect_functions: collector.effect_functions,
        }
    }

    pub(crate) fn uri(&self) -> &Uri {
        self.identity.uri()
    }

    pub(crate) fn saved_path(&self) -> Option<&Path> {
        self.identity.saved_path()
    }

    pub(crate) fn project_relative_path(&self) -> Option<&str> {
        self.identity.project_relative_path()
    }
}

fn narrow_block_reference_spans(block_references: &mut [BlockReferenceSummary], text: &str) {
    let lines = text.lines().collect::<Vec<_>>();
    for reference in block_references {
        let line_index = reference
            .span
            .start
            .line()
            .saturating_sub(1)
            .try_into()
            .unwrap_or(usize::MAX);
        let Some(line) = lines.get(line_index).copied() else {
            continue;
        };
        let Some(block_start) = line.rfind(reference.block_id.as_str()) else {
            continue;
        };
        let block_end = block_start + reference.block_id.len();
        let start_column = u32::try_from(line[..block_start].chars().count())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        let end_column = u32::try_from(line[..block_end].chars().count()).unwrap_or(u32::MAX);
        let Ok(start) = SourcePosition::new(reference.span.start.line(), start_column) else {
            continue;
        };
        let Ok(end) = SourcePosition::new(reference.span.start.line(), end_column) else {
            continue;
        };
        reference.span = SourceSpan::new(reference.span.file.clone(), start, Some(end));
    }
}

fn unique_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut unique = Vec::new();
    for diagnostic in diagnostics {
        if !unique.contains(&diagnostic) {
            unique.push(diagnostic);
        }
    }

    unique
}
