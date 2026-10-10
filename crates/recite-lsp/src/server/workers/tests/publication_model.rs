//! A protocol reference model: URI ownership and versions, without kernels,
//! workspace generations, invalidation helpers or diagnostic projection calls.
use super::super::super::{text_sync::Documents, updates::Update};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, NumberOrString, Position, PublishDiagnosticsParams, Range,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
    VersionedTextDocumentIdentifier,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

mod tests;

const DOCUMENT_COUNT: usize = 3;

#[derive(Clone, Debug)]
enum FileAction {
    Write(bool),
    Delete,
}

fn file_actions() -> impl Strategy<Value = FileAction> {
    prop_oneof![
        any::<bool>().prop_map(FileAction::Write),
        Just(FileAction::Delete)
    ]
}

#[derive(Clone, Copy, Debug)]
enum ActionKind {
    Open,
    Replace,
    Append,
    Stale,
    Malformed,
    Close,
}

#[derive(Clone, Debug)]
struct Action {
    document: usize,
    kind: ActionKind,
    version: i32,
    broken: bool,
}

fn actions() -> impl Strategy<Value = Action> {
    (
        0..DOCUMENT_COUNT,
        prop_oneof![
            Just(ActionKind::Open),
            Just(ActionKind::Replace),
            Just(ActionKind::Append),
            Just(ActionKind::Stale),
            Just(ActionKind::Malformed),
            Just(ActionKind::Close),
        ],
        1i32..6,
        any::<bool>(),
    )
        .prop_map(|(document, kind, version, broken)| Action {
            document,
            kind,
            version,
            broken,
        })
}

#[derive(Debug)]
struct DocumentState {
    version: i32,
    text: String,
    broken: bool,
}

#[derive(Default, Debug)]
struct EditorModel(BTreeMap<usize, DocumentState>);

impl EditorModel {
    fn publications(
        &self,
        documents: impl IntoIterator<Item = usize>,
    ) -> BTreeMap<String, PublishDiagnosticsParams> {
        documents
            .into_iter()
            .map(|document| {
                let state = self.0.get(&document);
                let publication = expected_publication(
                    editor_uri(document),
                    state.is_some_and(|state| state.broken),
                    state.map(|state| state.version),
                );
                (publication.uri.as_str().to_owned(), publication)
            })
            .collect()
    }

    fn accept(&mut self, action: &Action) -> bool {
        match action.kind {
            ActionKind::Open => {
                if self.0.contains_key(&action.document) {
                    return false;
                }
                self.0.insert(action.document, action.document_state());
            }
            ActionKind::Replace | ActionKind::Append => {
                let Some(document) = self.0.get_mut(&action.document) else {
                    return false;
                };
                if action.version <= document.version {
                    return false;
                }
                if matches!(action.kind, ActionKind::Replace) {
                    *document = action.document_state();
                } else {
                    let first_line_end = document.text.find('\n').unwrap();
                    document.text.insert_str(first_line_end, "\n# kept");
                    document.version = action.version;
                }
            }
            ActionKind::Close => return self.0.remove(&action.document).is_some(),
            ActionKind::Stale | ActionKind::Malformed => return false,
        }
        true
    }
}

impl Action {
    fn document_state(&self) -> DocumentState {
        DocumentState {
            version: self.version,
            text: source(self.document, self.broken),
            broken: self.broken,
        }
    }

    fn update(&self) -> Update {
        let uri = editor_uri(self.document);
        match self.kind {
            ActionKind::Open => Update::Open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri,
                    "recite".to_owned(),
                    self.version,
                    source(self.document, self.broken),
                ),
            }),
            ActionKind::Close => Update::Close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
            }),
            kind => {
                let mut changes = vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: source(self.document, self.broken),
                }];
                let version = match kind {
                    ActionKind::Stale => 0,
                    ActionKind::Malformed => {
                        // A valid first edit followed by an impossible line
                        // must reject the whole transaction, including version.
                        changes.push(TextDocumentContentChangeEvent {
                            range: Some(Range::new(
                                Position::new(u32::MAX, 0),
                                Position::new(u32::MAX, 0),
                            )),
                            range_length: None,
                            text: "invalid".to_owned(),
                        });
                        100
                    }
                    ActionKind::Append => {
                        changes[0].range = Some(Range::new(
                            Position::new(0, u32::MAX),
                            Position::new(0, u32::MAX),
                        ));
                        changes[0].text = "\n# kept".to_owned();
                        self.version
                    }
                    _ => self.version,
                };
                Update::Change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version },
                    content_changes: changes,
                })
            }
        }
    }
}

fn editor_uri(document: usize) -> Uri {
    format!("untitled:property-{document}").parse().unwrap()
}

fn source(document: usize, broken: bool) -> String {
    let prefix = if broken { "oops\n" } else { "" };
    format!("{prefix}:: buffer_{document}\n")
}

fn expected_publication(uri: Uri, broken: bool, version: Option<i32>) -> PublishDiagnosticsParams {
    let diagnostics = if broken {
        [
            (
                "RECITE_PARSE001",
                "expected a Recite statement header or indented prose",
            ),
            ("RECITE_PARSE002", "statement appears before a block header"),
        ]
        .into_iter()
        .map(|(code, message)| Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(code.to_owned())),
            source: Some("recite".to_owned()),
            message: message.to_owned(),
            ..Diagnostic::default()
        })
        .collect()
    } else {
        Vec::new()
    };
    PublishDiagnosticsParams::new(uri, diagnostics, version)
}
