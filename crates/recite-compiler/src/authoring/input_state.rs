use std::collections::{BTreeMap, BTreeSet};

use recite_core::DocumentKey;

use super::input::{OpenDocument, SavedDocument};
use super::snapshot::DocumentLayer;
use super::state::AuthoringError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectiveDocument {
    pub(super) key: DocumentKey,
    pub(super) text: String,
    pub(super) layer: DocumentLayer,
    pub(super) version: Option<super::DocumentVersion>,
}

pub(super) fn unique_saved(
    documents: &[SavedDocument],
) -> Result<BTreeMap<DocumentKey, SavedDocument>, AuthoringError> {
    let mut unique = BTreeMap::new();
    for document in documents {
        if unique
            .insert(document.key().clone(), document.clone())
            .is_some()
        {
            return Err(AuthoringError::DuplicateSavedDocument {
                key: document.key().clone(),
            });
        }
    }
    Ok(unique)
}

pub(super) fn unique_open(
    documents: &[OpenDocument],
) -> Result<BTreeMap<DocumentKey, OpenDocument>, AuthoringError> {
    let mut unique = BTreeMap::new();
    for document in documents {
        if unique
            .insert(document.key().clone(), document.clone())
            .is_some()
        {
            return Err(AuthoringError::DuplicateOpenDocument {
                key: document.key().clone(),
            });
        }
    }
    Ok(unique)
}

pub(super) fn validate_overlay_versions(
    old: &BTreeMap<DocumentKey, OpenDocument>,
    new: &BTreeMap<DocumentKey, OpenDocument>,
) -> Result<(), AuthoringError> {
    for (key, incoming) in new {
        let Some(previous) = old.get(key) else {
            continue;
        };
        if incoming.version() == previous.version() {
            if incoming.text() != previous.text() {
                return Err(AuthoringError::OverlayVersionConflict {
                    key: key.clone(),
                    version: incoming.version(),
                });
            }
        } else if incoming.version() < previous.version() {
            return Err(AuthoringError::StaleOverlayVersion {
                key: key.clone(),
                previous: previous.version(),
                received: incoming.version(),
            });
        }
    }
    Ok(())
}

pub(super) fn effective_documents(
    saved: &BTreeMap<DocumentKey, SavedDocument>,
    open: &BTreeMap<DocumentKey, OpenDocument>,
) -> BTreeMap<DocumentKey, EffectiveDocument> {
    let mut documents = saved
        .iter()
        .map(|(key, document)| {
            (
                key.clone(),
                EffectiveDocument {
                    key: key.clone(),
                    text: document.text().to_owned(),
                    layer: DocumentLayer::Saved,
                    version: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (key, document) in open {
        documents.insert(
            key.clone(),
            EffectiveDocument {
                key: key.clone(),
                text: document.text().to_owned(),
                layer: DocumentLayer::Open,
                version: Some(document.version()),
            },
        );
    }
    documents
}

pub(super) fn changed_keys(
    old_saved: &BTreeMap<DocumentKey, SavedDocument>,
    old_open: &BTreeMap<DocumentKey, OpenDocument>,
    new_saved: &BTreeMap<DocumentKey, SavedDocument>,
    new_open: &BTreeMap<DocumentKey, OpenDocument>,
) -> BTreeSet<DocumentKey> {
    let keys = old_saved
        .keys()
        .chain(old_open.keys())
        .chain(new_saved.keys())
        .chain(new_open.keys());
    keys.filter(|key| {
        old_saved.get(*key) != new_saved.get(*key) || old_open.get(*key) != new_open.get(*key)
    })
    .cloned()
    .collect()
}
