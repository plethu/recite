use std::collections::BTreeMap;

use lsp_types::{TextDocumentContentChangeEvent, Uri};
use recite_core::Diagnostic;
use recite_parser::parse;

#[derive(Clone, Debug)]
pub(crate) struct OpenDocument {
    version: i32,
    text: String,
    diagnostics: Vec<Diagnostic>,
}

impl OpenDocument {
    pub(crate) fn version(&self) -> i32 {
        self.version
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[derive(Default)]
pub(crate) struct OpenDocumentStore {
    documents: BTreeMap<Uri, OpenDocument>,
}

impl OpenDocumentStore {
    pub(crate) fn open(&mut self, uri: Uri, version: i32, text: String) -> &OpenDocument {
        let document = parse_document(&uri, version, text);
        self.documents.entry(uri).insert_entry(document).into_mut()
    }

    pub(crate) fn change(
        &mut self,
        uri: &Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Option<&OpenDocument> {
        if !self.documents.contains_key(uri) {
            return None;
        }

        if self.is_stale(uri, version) {
            return self.documents.get(uri);
        }

        let text = full_text_change(changes)?;
        let document = parse_document(uri, version, text);
        Some(
            self.documents
                .entry(uri.clone())
                .insert_entry(document)
                .into_mut(),
        )
    }

    pub(crate) fn close(&mut self, uri: &Uri) -> bool {
        self.documents.remove(uri).is_some()
    }

    fn is_stale(&self, uri: &Uri, version: i32) -> bool {
        self.documents
            .get(uri)
            .is_some_and(|document| version <= document.version)
    }
}

fn parse_document(uri: &Uri, version: i32, text: String) -> OpenDocument {
    let parse = parse(uri.as_str(), text.as_str());
    OpenDocument {
        version,
        text,
        diagnostics: parse.diagnostics().to_vec(),
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
