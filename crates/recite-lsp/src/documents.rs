use std::collections::BTreeMap;

use lsp_types::{TextDocumentContentChangeEvent, Uri};
use recite_core::Diagnostic;

use crate::summary::{FileSummary, OpenFileIdentity};

#[derive(Clone, Debug)]
pub(crate) struct OpenDocument {
    identity: OpenFileIdentity,
    version: i32,
    text: String,
    summary: FileSummary,
}

impl OpenDocument {
    pub(crate) fn identity(&self) -> &OpenFileIdentity {
        &self.identity
    }

    pub(crate) fn version(&self) -> i32 {
        self.version
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.summary.diagnostics
    }

    pub(crate) fn summary(&self) -> &FileSummary {
        &self.summary
    }
}

#[derive(Default)]
pub(crate) struct OpenDocumentStore {
    documents: BTreeMap<Uri, OpenDocument>,
}

impl OpenDocumentStore {
    pub(crate) fn open(
        &mut self,
        identity: OpenFileIdentity,
        version: i32,
        text: String,
    ) -> OpenDocument {
        let document = parse_document(identity, version, text);
        self.documents
            .insert(document.identity.uri.clone(), document.clone());
        document
    }

    pub(crate) fn change(
        &mut self,
        identity: OpenFileIdentity,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> DocumentChangeResult {
        let uri = identity.uri.clone();
        if !self.documents.contains_key(&uri) {
            return DocumentChangeResult::Unopened;
        }

        if self.is_stale(&uri, version) {
            return DocumentChangeResult::Stale;
        }

        let Some(text) = full_text_change(changes) else {
            return DocumentChangeResult::Malformed;
        };
        let document = parse_document(identity, version, text);
        self.documents.insert(uri, document.clone());
        DocumentChangeResult::Accepted(Box::new(document))
    }

    pub(crate) fn refresh_identity(
        &mut self,
        identity: OpenFileIdentity,
    ) -> Option<OpenDocumentIdentityRefresh> {
        let existing = self.documents.get(&identity.uri)?;
        if existing.identity == identity {
            return Some(OpenDocumentIdentityRefresh {
                document: existing.clone(),
                identity_changed: false,
            });
        }

        let document = parse_document(identity.clone(), existing.version, existing.text.clone());
        self.documents
            .insert(identity.uri.clone(), document.clone());
        Some(OpenDocumentIdentityRefresh {
            document,
            identity_changed: true,
        })
    }

    pub(crate) fn close(&mut self, uri: &Uri) -> Option<OpenDocument> {
        self.documents.remove(uri)
    }

    pub(crate) fn documents(&self) -> impl Iterator<Item = &OpenDocument> {
        self.documents.values()
    }

    pub(crate) fn document(&self, uri: &Uri) -> Option<&OpenDocument> {
        self.documents.get(uri)
    }

    fn is_stale(&self, uri: &Uri, version: i32) -> bool {
        self.documents
            .get(uri)
            .is_some_and(|document| version <= document.version)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DocumentChangeResult {
    Accepted(Box<OpenDocument>),
    Stale,
    Malformed,
    Unopened,
}

#[derive(Clone, Debug)]
pub(crate) struct OpenDocumentIdentityRefresh {
    pub(crate) document: OpenDocument,
    pub(crate) identity_changed: bool,
}

fn parse_document(identity: OpenFileIdentity, version: i32, text: String) -> OpenDocument {
    let summary = FileSummary::from_text(
        crate::summary::FileIdentity::Open(identity.clone()),
        Some(version),
        text.as_str(),
    );
    OpenDocument {
        identity,
        version,
        text,
        summary,
    }
}

fn full_text_change(changes: Vec<TextDocumentContentChangeEvent>) -> Option<String> {
    let mut changes = changes.into_iter();
    let change = changes.next()?;
    if changes.next().is_some() || change.range.is_some() || change.range_length.is_some() {
        return None;
    }

    Some(change.text)
}
