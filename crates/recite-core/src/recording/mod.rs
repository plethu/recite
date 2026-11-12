//! Recording production state kept outside compiled assets and runtime data (spec §9.9).
//!
//! This module owns the item revision, the per-language sidecar and the handoff row codecs.
//! Joining compiled lines and catalogues into recording items belongs to the authoring layer.

mod csv;
mod handoff;
mod sidecar;

pub use handoff::{HANDOFF_COLUMNS, HandoffError, HandoffRow};
pub use sidecar::{RecordingSidecar, SidecarError, SidecarItem, StoredStatus, Take};

/// Revision prefix; a new identity encoding would use a new prefix.
const REVISION_PREFIX: &str = "r1-";

/// The revision of one recording item: what a recording was made against.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordingRevision(String);

impl RecordingRevision {
    /// Hash the text forms being recorded, the speaker and the direction pairs in declared order.
    #[must_use]
    pub fn of(text: &[&str], speaker: Option<&str>, direction: &[(String, String)]) -> Self {
        let identity = serde_json::json!({
            "text": text,
            "speaker": speaker,
            "direction": direction
                .iter()
                .map(|(key, value)| [key.as_str(), value.as_str()])
                .collect::<Vec<_>>(),
        });
        let hash = blake3::hash(identity.to_string().as_bytes());
        let hex = hash.to_hex();
        Self(format!("{REVISION_PREFIX}{}", &hex[..16]))
    }

    /// Accept a stored revision string, rejecting anything that is not a v1 revision.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let digits = value.strip_prefix(REVISION_PREFIX)?;
        (digits.len() == 16
            && digits
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
        .then(|| Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RecordingRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
