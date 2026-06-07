use std::fs;
use std::path::PathBuf;

use lsp_types::{Position, Uri};

use crate::position::span_to_range;
use crate::summary::FileSummary;
use crate::workspace::LspWorkspace;

#[derive(Clone, Debug)]
pub struct LspBenchmarkProbes {
    pub document: LspDocumentProbe,
    pub completion: LspPositionProbe,
    pub definition: LspPositionProbe,
    pub rename: LspPositionProbe,
}

impl LspBenchmarkProbes {
    pub(crate) fn discover(workspace: &LspWorkspace) -> Self {
        let summaries = workspace.snapshot().summaries();
        let document = summaries
            .iter()
            .find_map(LspDocumentProbe::from_summary)
            .unwrap_or_else(|| {
                panic!("LSP benchmark fixture contains at least one saved source file")
            });
        let completion = summaries
            .iter()
            .find_map(block_reference_probe)
            .unwrap_or_else(|| {
                panic!("LSP benchmark fixture contains at least one block reference")
            });
        let definition = completion.clone();
        let rename = summaries
            .iter()
            .find_map(block_definition_probe)
            .unwrap_or_else(|| {
                panic!("LSP benchmark fixture contains at least one block definition")
            });

        Self {
            document,
            completion,
            definition,
            rename,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LspDocumentProbe {
    pub uri: Uri,
    pub path: PathBuf,
    pub project_relative_path: String,
}

impl LspDocumentProbe {
    fn from_summary(summary: &FileSummary) -> Option<Self> {
        Some(Self {
            uri: summary.uri().clone(),
            path: summary.saved_path()?.to_owned(),
            project_relative_path: summary.project_relative_path()?.to_owned(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct LspPositionProbe {
    pub uri: Uri,
    pub path: PathBuf,
    pub project_relative_path: String,
    pub position: Position,
}

pub(crate) fn read_probe_text_or_panic(probe: &LspDocumentProbe) -> String {
    match fs::read_to_string(&probe.path) {
        Ok(text) => text,
        Err(error) => panic!(
            "failed to read LSP benchmark probe `{}`: {error}",
            probe.path.display()
        ),
    }
}

fn block_reference_probe(summary: &FileSummary) -> Option<LspPositionProbe> {
    let reference = summary.block_references.first()?;
    let text = read_summary_text(summary)?;
    let range = span_to_range(&text, &reference.span);
    position_probe(summary, range.end)
}

fn block_definition_probe(summary: &FileSummary) -> Option<LspPositionProbe> {
    let block = summary.blocks.first()?;
    let text = read_summary_text(summary)?;
    let full_range = span_to_range(&text, &block.span);
    let line = text
        .lines()
        .nth(usize::try_from(full_range.start.line).ok()?)
        .unwrap_or_default();
    let start_byte = line.find(block.name.as_str())?;
    let position = Position {
        line: full_range.start.line,
        character: utf16_units_for_byte_index(line, start_byte),
    };
    position_probe(summary, position)
}

fn position_probe(summary: &FileSummary, position: Position) -> Option<LspPositionProbe> {
    Some(LspPositionProbe {
        uri: summary.uri().clone(),
        path: summary.saved_path()?.to_owned(),
        project_relative_path: summary.project_relative_path()?.to_owned(),
        position,
    })
}

fn read_summary_text(summary: &FileSummary) -> Option<String> {
    fs::read_to_string(summary.saved_path()?).ok()
}

fn utf16_units_for_byte_index(line: &str, byte_index: usize) -> u32 {
    line[..byte_index]
        .chars()
        .map(char::len_utf16)
        .fold(0u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        })
}
