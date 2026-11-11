//! The per-language recording sidecar: stored status and returned takes keyed by stable ID.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::RecordingRevision;

const FORMAT_VERSION: u32 = 1;

/// Status a person or an import stored for one item. Reports derive the rest (spec §9.9).
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredStatus {
    Recorded,
    Approved,
    Unvoiced,
}

/// One returned recording, kept with the revision it was made against.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Take {
    pub reference: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarItem {
    pub status: StoredStatus,
    /// The revision `recorded` or `approved` applies to; `unvoiced` items need none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub takes: Vec<Take>,
}

impl SidecarItem {
    #[must_use]
    pub fn revision(&self) -> Option<RecordingRevision> {
        self.revision.as_deref().and_then(RecordingRevision::parse)
    }
}

/// `recording/source.toml` or `recording/<locale>.toml`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordingSidecar {
    /// `None` for the source language.
    pub locale: Option<String>,
    pub items: BTreeMap<String, SidecarItem>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawSidecar {
    format_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    locale: Option<String>,
    #[serde(default)]
    items: BTreeMap<String, SidecarItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("{0}")]
    Toml(String),
    #[error("unsupported recording sidecar format_version {0}; expected 1")]
    FormatVersion(u32),
    #[error("item `{id}` has status `{status}` but no valid `r1-` revision")]
    MissingRevision { id: String, status: &'static str },
    #[error("item `{id}` has a take with an invalid revision `{revision}`")]
    TakeRevision { id: String, revision: String },
}

impl RecordingSidecar {
    #[must_use]
    pub fn new(locale: Option<String>) -> Self {
        Self {
            locale,
            items: BTreeMap::new(),
        }
    }

    pub fn parse(source: &str) -> Result<Self, SidecarError> {
        let raw: RawSidecar =
            toml::from_str(source).map_err(|error| SidecarError::Toml(error.to_string()))?;
        if raw.format_version != FORMAT_VERSION {
            return Err(SidecarError::FormatVersion(raw.format_version));
        }
        for (id, item) in &raw.items {
            let status = match item.status {
                StoredStatus::Recorded => Some("recorded"),
                StoredStatus::Approved => Some("approved"),
                StoredStatus::Unvoiced => None,
            };
            if let Some(status) = status
                && item.revision().is_none()
            {
                return Err(SidecarError::MissingRevision {
                    id: id.clone(),
                    status,
                });
            }
            if let Some(take) = item
                .takes
                .iter()
                .find(|take| RecordingRevision::parse(&take.revision).is_none())
            {
                return Err(SidecarError::TakeRevision {
                    id: id.clone(),
                    revision: take.revision.clone(),
                });
            }
        }
        Ok(Self {
            locale: raw.locale,
            items: raw.items,
        })
    }

    /// Deterministic TOML: items sorted by stable ID, takes in the order they were returned.
    #[must_use]
    pub fn to_toml_string(&self) -> String {
        let raw = RawSidecar {
            format_version: FORMAT_VERSION,
            locale: self.locale.clone(),
            items: self.items.clone(),
        };
        toml::to_string(&raw).unwrap_or_else(|error| {
            unreachable!("recording sidecar values always serialize: {error}")
        })
    }
}
