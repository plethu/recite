use std::fmt::Write as _;

use recite_core::DocumentKey;

use super::project_index::SavedDocument;
use crate::documents::OpenDocument;
use crate::paths::stable_path_identity;
use crate::summary::{FileIdentity, OpenFileScope};

pub(crate) fn document_key_for_saved(document: &SavedDocument) -> Option<DocumentKey> {
    document_key_for_identity(&FileIdentity::Saved(document.identity.clone()))
}

pub(crate) fn document_key_for_open(document: &OpenDocument) -> Option<DocumentKey> {
    document_key_for_identity(&FileIdentity::Open(document.identity().clone()))
}

pub(crate) fn document_key_for_identity(identity: &FileIdentity) -> Option<DocumentKey> {
    match identity {
        FileIdentity::Saved(identity) => document_key(identity.project_relative_path.as_str())
            .or_else(|| document_key(&stable_path_identity(&identity.canonical_path))),
        FileIdentity::Open(identity) if identity.scope == OpenFileScope::Excluded => None,
        FileIdentity::Open(identity) => identity
            .project_relative_path
            .as_deref()
            .and_then(document_key)
            .or_else(|| {
                identity
                    .saved_path
                    .as_deref()
                    .map(stable_path_identity)
                    .and_then(|path| document_key(&path))
            })
            .or_else(|| fallback_document_key(identity.uri.as_str().as_bytes())),
    }
}

fn document_key(value: &str) -> Option<DocumentKey> {
    DocumentKey::new(value.to_owned()).ok()
}

fn fallback_document_key(value: &[u8]) -> Option<DocumentKey> {
    let mut encoded = String::from("~lsp/");
    for byte in value {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    document_key(&encoded)
}

pub(super) fn standalone_document_key(document: &OpenDocument) -> Option<DocumentKey> {
    document
        .identity()
        .saved_path
        .as_deref()
        .map(stable_path_identity)
        .and_then(|path| document_key(&path))
        .or_else(|| fallback_document_key(document.identity().uri.as_str().as_bytes()))
}
