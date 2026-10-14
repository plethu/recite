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
            schema_stale: current_schema
                .is_some_and(|current| asset.header.schema_fingerprint != current),
        }
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
