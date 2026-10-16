//! Immutable field text; contiguous access is an explicit materialization boundary.
use ropey::Rope;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

#[derive(Clone)]
pub struct DraftSnapshot(Contents);

#[derive(Clone)]
enum Contents {
    Text(Arc<String>),
    Shared(Arc<str>),
    Rope(Arc<RopeDraft>),
}

struct RopeDraft {
    source: Rope,
    text: OnceLock<String>,
}

impl DraftSnapshot {
    pub fn from_rope(source: Rope) -> Self {
        Self(Contents::Rope(Arc::new(RopeDraft {
            source,
            text: OnceLock::new(),
        })))
    }

    /// Explicit consumers share one exact-capacity materialization per snapshot.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Contents::Text(text) => text.as_str(),
            Contents::Shared(text) => text.as_ref(),
            Contents::Rope(draft) => draft
                .text
                .get_or_init(|| String::from(&draft.source))
                .as_str(),
        }
    }

    pub fn matches_rope(&self, source: &Rope) -> bool {
        match &self.0 {
            Contents::Text(text) => source == text.as_str(),
            Contents::Shared(text) => source == text.as_ref(),
            Contents::Rope(draft) => draft.source.is_instance(source) || &draft.source == source,
        }
    }
}

impl From<String> for DraftSnapshot {
    fn from(text: String) -> Self {
        Self(Contents::Text(Arc::new(text)))
    }
}

impl From<Arc<str>> for DraftSnapshot {
    fn from(text: Arc<str>) -> Self {
        Self(Contents::Shared(text))
    }
}

impl PartialEq for DraftSnapshot {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Contents::Text(left), Contents::Text(right)) => {
                Arc::ptr_eq(left, right) || left == right
            }
            (Contents::Shared(left), Contents::Shared(right)) => {
                Arc::ptr_eq(left, right) || left == right
            }
            (Contents::Text(text), Contents::Shared(shared))
            | (Contents::Shared(shared), Contents::Text(text)) => text.as_str() == shared.as_ref(),
            (Contents::Rope(left), Contents::Rope(right)) => {
                Arc::ptr_eq(left, right)
                    || left.source.is_instance(&right.source)
                    || left.source == right.source
            }
            (Contents::Text(text), Contents::Rope(draft))
            | (Contents::Rope(draft), Contents::Text(text)) => draft.source == text.as_str(),
            (Contents::Shared(text), Contents::Rope(draft))
            | (Contents::Rope(draft), Contents::Shared(text)) => draft.source == text.as_ref(),
        }
    }
}

impl Eq for DraftSnapshot {}

impl fmt::Debug for DraftSnapshot {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Contents::Text(text) => fmt::Debug::fmt(text, output),
            Contents::Shared(text) => fmt::Debug::fmt(text, output),
            Contents::Rope(draft) => fmt::Debug::fmt(&draft.source, output),
        }
    }
}

impl Serialize for DraftSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.0 {
            Contents::Text(text) => serializer.serialize_str(text.as_str()),
            Contents::Shared(text) => serializer.serialize_str(text.as_ref()),
            // Rope's Display visits its UTF-8 chunks; serde_json streams and
            // escapes them inside one ordinary JSON string.
            Contents::Rope(draft) => serializer.collect_str(&draft.source),
        }
    }
}

impl<'de> Deserialize<'de> for DraftSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests;
