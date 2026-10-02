//! Source identity avoids rescanning unchanged project text. Separately owned
//! but equal text retains the same value equality as derived implementations.
use std::sync::Arc;

use super::{OpenDocument, SavedDocument};

impl PartialEq for SavedDocument {
    fn eq(&self, other: &Self) -> bool {
        let Self { key, text } = self;
        let Self {
            key: other_key,
            text: other_text,
        } = other;
        key == other_key && (Arc::ptr_eq(text, other_text) || text == other_text)
    }
}

impl PartialEq for OpenDocument {
    fn eq(&self, other: &Self) -> bool {
        let Self { key, version, text } = self;
        let Self {
            key: other_key,
            version: other_version,
            text: other_text,
        } = other;
        key == other_key
            && version == other_version
            && (Arc::ptr_eq(text, other_text) || text == other_text)
    }
}
