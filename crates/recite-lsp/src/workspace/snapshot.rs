use std::collections::{BTreeMap, BTreeSet};

use recite_compiler::AuthoringSnapshot;
use recite_core::DocumentKey;

use super::SnapshotGeneration;
use super::project_index::SavedProjectIndex;
use crate::documents::OpenDocumentStore;
use crate::summary::{FileIdentity, FileSummary};

pub(crate) struct LiveProjectSnapshot {
    generation: SnapshotGeneration,
    summaries: Vec<FileSummary>,
}

impl LiveProjectSnapshot {
    pub(super) fn empty(generation: SnapshotGeneration) -> Self {
        Self {
            generation,
            summaries: Vec::new(),
        }
    }

    pub(super) fn rebuild(
        generation: SnapshotGeneration,
        saved: &SavedProjectIndex,
        documents: &OpenDocumentStore,
        kernel: &AuthoringSnapshot,
    ) -> Self {
        let mut identities = BTreeMap::<DocumentKey, FileIdentity>::new();
        for document in saved.documents.values() {
            if let Some(key) = super::document_key_for_saved(document) {
                identities.insert(key, FileIdentity::Saved(document.identity.clone()));
            }
        }
        let mut open_keys = BTreeSet::new();
        for document in documents.documents() {
            let Some(key) = super::document_key_for_open(document) else {
                continue;
            };
            // URI iteration is deterministic. Keep the first alias for one
            // canonical key so the kernel and every projection have one
            // effective document.
            if open_keys.insert(key.clone()) {
                identities.insert(key, FileIdentity::Open(document.identity().clone()));
            }
        }

        let mut summaries = kernel
            .documents()
            .iter()
            .filter_map(|document| {
                let identity = identities.get(document.key())?.clone();
                let version = document
                    .version()
                    .and_then(|version| i32::try_from(version.as_i64()).ok());
                Some(FileSummary::from_authoring(identity, version, document))
            })
            .collect::<Vec<_>>();
        summaries.sort_by(summary_sort_key);

        Self {
            generation,
            summaries,
        }
    }

    pub(crate) fn generation(&self) -> SnapshotGeneration {
        self.generation
    }

    pub(crate) fn summaries(&self) -> &[FileSummary] {
        &self.summaries
    }
}

fn summary_sort_key(left: &FileSummary, right: &FileSummary) -> std::cmp::Ordering {
    let left_path = left.project_relative_path().unwrap_or(left.uri().as_str());
    let right_path = right
        .project_relative_path()
        .unwrap_or(right.uri().as_str());
    left_path
        .cmp(right_path)
        .then_with(|| left.uri().cmp(right.uri()))
}
