use recite_core::{CompiledAssetId, CompilerVersion, SourceMapId};

use crate::{DialogueSchemaFingerprintSnapshot, DialogueSessionSourceSnapshot};

/// The canonical build identity used to distinguish compiled payload revisions.
/// It reuses the asset header and source fingerprints already checked by the
/// session runtime; it is not a hash of a debug representation.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewAssetRevision {
    asset_id: CompiledAssetId,
    format_version: u16,
    compiler_compatibility_version: u16,
    compiler_version: CompilerVersion,
    source_map_id: SourceMapId,
    schema_fingerprint: DialogueSchemaFingerprintSnapshot,
    sources: Vec<DialogueSessionSourceSnapshot>,
}

impl PreviewAssetRevision {
    pub(crate) fn from_asset(asset: &recite_core::CompiledDialogue) -> Self {
        Self {
            asset_id: asset.header.asset_id.clone(),
            format_version: asset.header.format_version,
            compiler_compatibility_version: asset.header.compiler_compatibility_version,
            compiler_version: asset.header.compiler_version.clone(),
            source_map_id: asset.header.source_map_id.clone(),
            schema_fingerprint: crate::schema_fingerprint_snapshot(
                &asset.header.schema_fingerprint,
            ),
            sources: asset.sources.iter().map(crate::source_snapshot).collect(),
        }
    }

    pub(crate) fn from_parts(
        asset_id: CompiledAssetId,
        format_version: u16,
        compiler_compatibility_version: u16,
        compiler_version: CompilerVersion,
        source_map_id: SourceMapId,
        schema_fingerprint: DialogueSchemaFingerprintSnapshot,
        sources: Vec<DialogueSessionSourceSnapshot>,
    ) -> Self {
        Self {
            asset_id,
            format_version,
            compiler_compatibility_version,
            compiler_version,
            source_map_id,
            schema_fingerprint,
            sources,
        }
    }

    pub(crate) fn format_version(&self) -> u16 {
        self.format_version
    }

    pub(crate) fn compiler_compatibility_version(&self) -> u16 {
        self.compiler_compatibility_version
    }

    pub(crate) fn compiler_version(&self) -> &CompilerVersion {
        &self.compiler_version
    }

    pub(crate) fn source_map_id(&self) -> &SourceMapId {
        &self.source_map_id
    }

    pub(crate) fn schema_fingerprint(&self) -> &DialogueSchemaFingerprintSnapshot {
        &self.schema_fingerprint
    }

    #[must_use]
    pub fn asset_id(&self) -> &CompiledAssetId {
        &self.asset_id
    }

    #[must_use]
    pub fn sources(&self) -> &[DialogueSessionSourceSnapshot] {
        &self.sources
    }
}
