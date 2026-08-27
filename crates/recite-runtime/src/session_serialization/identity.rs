use recite_core::CompiledDialogue;

use crate::DialogueError;
use crate::session_snapshot::{
    DialogueSessionSnapshot, schema_fingerprint_snapshot, source_snapshot,
};

pub(super) fn ensure_snapshot_matches_asset(
    asset: &CompiledDialogue,
    snapshot: &DialogueSessionSnapshot,
) -> Result<(), DialogueError> {
    let actual_schema_fingerprint = schema_fingerprint_snapshot(&asset.header.schema_fingerprint);
    if snapshot.schema_fingerprint != actual_schema_fingerprint {
        return Err(DialogueError::SchemaMismatch {
            asset_id: snapshot.asset_id.clone(),
            expected_schema_fingerprint: snapshot.schema_fingerprint.clone(),
            actual_schema_fingerprint,
        });
    }
    if snapshot.asset_id != asset.header.asset_id.as_str()
        || snapshot.asset_format_version != asset.header.format_version
        || snapshot.asset_compiler_compatibility_version
            != asset.header.compiler_compatibility_version
    {
        return Err(DialogueError::AssetMismatch {
            expected_asset_id: snapshot.asset_id.clone(),
            actual_asset_id: asset.header.asset_id.as_str().to_owned(),
            expected_format_version: snapshot.asset_format_version,
            actual_format_version: asset.header.format_version,
            expected_compiler_compatibility_version: snapshot.asset_compiler_compatibility_version,
            actual_compiler_compatibility_version: asset.header.compiler_compatibility_version,
        });
    }
    if snapshot.compiler_version != asset.header.compiler_version.as_str() {
        return asset_content_mismatch(
            snapshot,
            "compiler version differs from the provided compiled asset",
        );
    }
    if snapshot.source_map_id != asset.header.source_map_id.as_str() {
        return asset_content_mismatch(
            snapshot,
            "source map id differs from the provided compiled asset",
        );
    }
    let sources = asset
        .sources
        .iter()
        .map(source_snapshot)
        .collect::<Vec<_>>();
    if snapshot.sources != sources {
        return asset_content_mismatch(
            snapshot,
            "source fingerprints differ from the provided compiled asset",
        );
    }

    Ok(())
}

fn asset_content_mismatch<T>(
    snapshot: &DialogueSessionSnapshot,
    reason: impl Into<String>,
) -> Result<T, DialogueError> {
    Err(DialogueError::AssetContentMismatch {
        asset_id: snapshot.asset_id.clone(),
        reason: reason.into(),
    })
}
