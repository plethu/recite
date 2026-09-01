use std::fs;
use std::path::PathBuf;

use lsp_types::{Position, Uri};
use recite_compiler::DocumentSnapshot;

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
            .find_map(|summary| block_reference_probe(workspace, summary))
            .unwrap_or_else(|| {
                panic!("LSP benchmark fixture contains at least one block reference")
            });
        let definition = summaries
            .iter()
            .find_map(|summary| block_reference_definition_probe(workspace, summary))
            .unwrap_or_else(|| {
                panic!("LSP benchmark fixture contains at least one block reference")
            });
        let rename = summaries
            .iter()
            .find_map(|summary| block_definition_probe(workspace, summary))
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

fn block_reference_probe(
    workspace: &LspWorkspace,
    summary: &FileSummary,
) -> Option<LspPositionProbe> {
    let document = compiler_document(workspace, summary)?;
    let reference = document.summary().block_references().first()?;
    let text = read_summary_text(summary)?;
    let range = span_to_range(&text, reference.block_id_span().unwrap_or(reference.span()));
    position_probe(summary, range.end)
}

fn block_reference_definition_probe(
    workspace: &LspWorkspace,
    summary: &FileSummary,
) -> Option<LspPositionProbe> {
    let document = compiler_document(workspace, summary)?;
    let reference = document.summary().block_references().first()?;
    let text = read_summary_text(summary)?;
    let range = span_to_range(&text, reference.block_id_span().unwrap_or(reference.span()));
    position_probe(summary, range.start)
}

fn block_definition_probe(
    workspace: &LspWorkspace,
    summary: &FileSummary,
) -> Option<LspPositionProbe> {
    let document = compiler_document(workspace, summary)?;
    let block = document.summary().blocks().first()?;
    let text = read_summary_text(summary)?;
    let position = span_to_range(&text, block.id_span().unwrap_or(block.span())).start;
    position_probe(summary, position)
}

fn compiler_document<'a>(
    workspace: &'a LspWorkspace,
    summary: &FileSummary,
) -> Option<&'a DocumentSnapshot> {
    workspace.compiler_document_for_summary(summary)
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
