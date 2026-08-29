use std::collections::BTreeSet;
use std::path::Path;

use super::SnapshotGeneration;
use super::project_index::SavedProjectIndex;
use crate::documents::OpenDocumentStore;
use crate::summary::FileSummary;

pub(crate) struct LiveProjectSnapshot {
    #[allow(
        dead_code,
        reason = "snapshot generation is exposed only for LSP lifecycle assertions"
    )]
    generation: SnapshotGeneration,
    summaries: Vec<FileSummary>,
}

impl LiveProjectSnapshot {
    pub(super) fn rebuild(
        generation: SnapshotGeneration,
        saved: &SavedProjectIndex,
        documents: &OpenDocumentStore,
    ) -> Self {
        let open_saved_paths = documents
            .documents()
            .filter_map(|document| document.summary().saved_path().map(Path::to_owned))
            .collect::<BTreeSet<_>>();
        let open_uris = documents
            .documents()
            .map(|document| document.summary().uri().as_str().to_owned())
            .collect::<BTreeSet<_>>();

        let mut summaries = saved
            .documents
            .iter()
            .filter(|(path, document)| {
                !open_saved_paths.contains(*path)
                    && !open_uris.contains(document.summary.uri().as_str())
            })
            .map(|(_, document)| document.summary.clone())
            .collect::<Vec<_>>();
        summaries.extend(
            documents
                .documents()
                .map(|document| document.summary().clone()),
        );
        summaries.sort_by(summary_sort_key);

        Self {
            generation,
            summaries,
        }
    }

    #[allow(
        dead_code,
        reason = "snapshot generation is exposed only for LSP lifecycle assertions"
    )]
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
