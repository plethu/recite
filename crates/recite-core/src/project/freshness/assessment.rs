//! Per-asset work is shared only within one filesystem freshness assessment.
use super::super::{ProjectManifestSource, ProjectScene};
use crate::{
    Diagnostic,
    compiled::{
        CompiledDialogue, CompiledSourceFile, SchemaFingerprint, canonical_source_fingerprint,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) enum SourceIssue {
    Stale,
    Missing,
}

/// Borrowed asset freshness checks reusable for scenes sharing that asset.
/// Current source text is fingerprinted once and is not retained.
pub struct ProjectAssetFreshness<'a> {
    pub(super) asset: &'a CompiledDialogue,
    pub(super) speakers: BTreeSet<&'a str>,
    pub(super) source_issues: Vec<(&'a CompiledSourceFile, SourceIssue)>,
    /// Project sources the asset was not built from, by project-relative path.
    pub(super) added_sources: Vec<String>,
    pub(super) schema_stale: bool,
}

impl<'a> ProjectAssetFreshness<'a> {
    #[must_use]
    pub fn new(
        asset: &'a CompiledDialogue,
        current_sources: &BTreeMap<&str, Option<&str>>,
        current_schema: Option<SchemaFingerprint>,
    ) -> Self {
        let source_issues = asset
            .sources
            .iter()
            .filter_map(|source| {
                let issue = match current_sources.get(source.path.as_str()) {
                    Some(Some(current))
                        if canonical_source_fingerprint(current) == source.fingerprint =>
                    {
                        return None;
                    }
                    Some(Some(_)) => SourceIssue::Stale,
                    Some(None) | None => SourceIssue::Missing,
                };
                Some((source, issue))
            })
            .collect();
        Self {
            asset,
            speakers: asset
                .speakers
                .iter()
                .map(|speaker| speaker.id.as_str())
                .collect(),
            source_issues,
            added_sources: Vec::new(),
            schema_stale: current_schema
                .is_some_and(|current| asset.header.schema_fingerprint != current),
        }
    }

    /// Record project sources that exist now but were not part of the build; they make the asset
    /// stale just as a changed source does.
    #[must_use]
    pub fn with_added_sources(mut self, added: impl IntoIterator<Item = String>) -> Self {
        self.added_sources = added.into_iter().collect();
        self.added_sources.sort();
        self
    }

    /// Render scene-specific diagnostics in their existing order and source spans.
    #[must_use]
    pub fn validate_scene_source(
        &self,
        source: &ProjectManifestSource,
        scene_index: usize,
        scene: &ProjectScene,
    ) -> Vec<Diagnostic> {
        super::validate_scene(
            source.file(),
            source.source_text_ref(),
            Some(source.spans_ref()),
            scene_index,
            scene,
            self,
        )
    }
}
