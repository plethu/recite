//! Prose findings for open documents in projects with `[prose] enabled`.
//!
//! The shared `recite-prose` crate owns settings, the engine and fix ranges;
//! this module only selects the project, caches results per document text and
//! projects findings into diagnostics and code actions.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CreateFile,
    CreateFileOptions, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Range, ResourceOp, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit,
};
use recite_compiler::authoring::SourceRange;
use recite_core::{Diagnostic, DiagnosticSeverity, SourceLineIndex};
use recite_prose::{
    ProseChecker, ProseCheckerCache, ProseFinding, ProseFindingKind, ProseSettings,
    dictionary_addition,
};
use recite_ui::MsgId;

use super::{LspWorkspace, document_key_for_open};
use crate::documents::OpenDocument;
use crate::features::code_action::{includes_quick_fix, ranges_intersect};
use crate::position::{source_range_to_lsp, span_to_range};

/// Checkers per project and the latest findings per open document.
#[derive(Debug, Default)]
pub(crate) struct ProseService {
    checkers: ProseCheckerCache,
    results: Mutex<BTreeMap<String, Cached>>,
}

#[derive(Debug)]
struct Cached {
    text: String,
    checker: Arc<Mutex<ProseChecker>>,
    findings: Arc<Vec<ProseFinding>>,
}

/// Findings for one open document, or the reason prose cannot be checked.
pub(crate) enum DocumentProse {
    Findings {
        settings: ProseSettings,
        findings: Arc<Vec<ProseFinding>>,
    },
    Setup(Box<Diagnostic>),
}

impl LspWorkspace {
    /// `None` when the document's project does not enable prose checking.
    pub(crate) fn document_prose(&self, document: &OpenDocument) -> Option<DocumentProse> {
        let partition = self.partition_id_for_open(document)?;
        let manifest = self.saved.manifest_for_partition(&partition)?;
        let prose = &manifest.source().manifest().prose;
        if !prose.enabled {
            return None;
        }
        let key = document_key_for_open(document)?;
        let root = manifest.project_root();
        let setup = |error: recite_prose::ProseSetupError| {
            let mut diagnostic = error.diagnostic(key.as_str())?;
            diagnostic.severity = DiagnosticSeverity::Warning;
            Some(DocumentProse::Setup(Box::new(diagnostic)))
        };
        let settings = match ProseSettings::from_project(root, prose) {
            Ok(settings) => settings,
            Err(error) => return setup(error),
        };
        let checker = match self.prose.checkers.checker(root, &settings) {
            Ok(checker) => checker,
            Err(error) => return setup(error),
        };
        let findings = self.prose.findings(
            &document.identity().uri,
            key.as_str(),
            document.text(),
            checker,
        );
        Some(DocumentProse::Findings { settings, findings })
    }

    /// Prose diagnostics appended to an open document's publication.
    pub(crate) fn prose_diagnostics(&self, document: &OpenDocument) -> Vec<Diagnostic> {
        match self.document_prose(document) {
            Some(DocumentProse::Findings { findings, .. }) => findings
                .iter()
                .map(|finding| finding.diagnostic.clone())
                .collect(),
            Some(DocumentProse::Setup(diagnostic)) => vec![*diagnostic],
            None => Vec::new(),
        }
    }

    /// Replacement and add-to-dictionary quick fixes for findings in range.
    pub(crate) fn prose_code_actions(&self, params: &CodeActionParams) -> Vec<CodeActionOrCommand> {
        if !includes_quick_fix(params) {
            return Vec::new();
        }
        let Some(document) = self.documents.document(&params.text_document.uri) else {
            return Vec::new();
        };
        let Some(DocumentProse::Findings { settings, findings }) = self.document_prose(document)
        else {
            return Vec::new();
        };
        let text = document.text();
        let index = SourceLineIndex::new(text);
        let mut actions = Vec::new();
        for finding in findings.iter() {
            let range = span_to_range(text, &finding.diagnostic.span);
            if !ranges_intersect(params.range, range) {
                continue;
            }
            for fix in &finding.fixes {
                let Some(range) = byte_range_to_lsp(&index, fix.range.clone()) else {
                    continue;
                };
                let edit = TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: params.text_document.uri.clone(),
                        version: Some(document.version()),
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range,
                        new_text: fix.replacement.clone(),
                    })],
                };
                actions.push(action(
                    self.ui_catalog.format_pairs(
                        MsgId::LspCodeActionProseReplace,
                        [("replacement", fix.replacement.as_str())],
                    ),
                    params,
                    DocumentChanges::Edits(vec![edit]),
                ));
            }
            if finding.kind == ProseFindingKind::Spelling
                && let Some(changes) = settings
                    .dictionary_for_additions()
                    .and_then(|path| add_word_changes(path.to_owned(), &finding.text))
            {
                actions.push(action(
                    self.ui_catalog.format_pairs(
                        MsgId::LspCodeActionProseAddWord,
                        [("word", finding.text.as_str())],
                    ),
                    params,
                    changes,
                ));
            }
        }
        actions
    }
}

impl ProseService {
    fn findings(
        &self,
        uri: &Uri,
        path: &str,
        text: &str,
        checker: Arc<Mutex<ProseChecker>>,
    ) -> Arc<Vec<ProseFinding>> {
        let mut results = self.results.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(cached) = results.get(uri.as_str())
            && cached.text == text
            && Arc::ptr_eq(&cached.checker, &checker)
        {
            return cached.findings.clone();
        }
        let findings = Arc::new(
            checker
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .check(path, text),
        );
        results.insert(
            uri.as_str().to_owned(),
            Cached {
                text: text.to_owned(),
                checker,
                findings: findings.clone(),
            },
        );
        findings
    }
}

fn action(
    title: String,
    params: &CodeActionParams,
    changes: DocumentChanges,
) -> CodeActionOrCommand {
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(params.context.diagnostics.clone()),
        edit: Some(WorkspaceEdit {
            changes: None,
            document_changes: Some(changes),
            change_annotations: None,
        }),
        ..CodeAction::default()
    })
}

fn byte_range_to_lsp(index: &SourceLineIndex, range: std::ops::Range<usize>) -> Option<Range> {
    let start = index.position_for_byte_offset(range.start)?;
    let end = index.position_for_byte_offset(range.end)?;
    source_range_to_lsp(index, SourceRange::new(start, end))
}

/// Append a word to the project dictionary, creating the file when needed.
/// The edit targets the file on disk; an editor holding it open with unsaved
/// changes rejects the stale edit rather than overwriting them.
fn add_word_changes(path: PathBuf, word: &str) -> Option<DocumentChanges> {
    let uri = crate::paths::file_path_to_uri(&path)?;
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return None,
    };
    let edit = dictionary_addition(existing.as_deref(), word)?;
    let index = SourceLineIndex::new(existing.clone().unwrap_or_default());
    let range = byte_range_to_lsp(&index, edit.range)?;
    let text_edit = TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: None,
        },
        edits: vec![OneOf::Left(TextEdit {
            range,
            new_text: edit.replacement,
        })],
    };
    Some(match existing {
        Some(_) => DocumentChanges::Edits(vec![text_edit]),
        None => DocumentChanges::Operations(vec![
            DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                uri,
                options: Some(CreateFileOptions {
                    overwrite: Some(false),
                    ignore_if_exists: Some(true),
                }),
                annotation_id: None,
            })),
            DocumentChangeOperation::Edit(text_edit),
        ]),
    })
}
