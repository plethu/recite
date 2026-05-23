use std::collections::BTreeSet;

use super::{
    MALFORMED_COMPILED_ASSET, MISSING_SOURCE_ASSET, ProjectFreshnessInput,
    STALE_COMPILER_COMPATIBILITY, STALE_SCHEMA_FINGERPRINT, STALE_SOURCE_FINGERPRINT,
    UNKNOWN_PARTICIPANT, UNKNOWN_START_BLOCK,
    spans::{diagnostic, scene_key_span},
};
use crate::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, Diagnostic,
    canonical_source_fingerprint,
};

/// Validate one decoded asset against the project scene and current source/schema state.
#[must_use]
pub fn validate_project_freshness(
    file: &str,
    source: &str,
    input: ProjectFreshnessInput<'_>,
) -> Vec<Diagnostic> {
    let scene = input.scene;
    let mut diagnostics = Vec::new();

    if !input
        .asset
        .block_lookup
        .iter()
        .any(|entry| entry.id.as_str() == scene.block)
    {
        diagnostics.push(diagnostic(
            UNKNOWN_START_BLOCK,
            format!(
                "scene '{}' references unknown block '{}'",
                scene.id, scene.block
            ),
            scene_key_span(file, source, input.scene_index, "block"),
        ));
    }

    let asset_speakers = input
        .asset
        .speakers
        .iter()
        .map(|speaker| speaker.id.as_str())
        .collect::<BTreeSet<_>>();
    for participant in &scene.participants {
        if !asset_speakers.is_empty() && !asset_speakers.contains(participant.as_str()) {
            diagnostics.push(diagnostic(
                UNKNOWN_PARTICIPANT,
                format!(
                    "scene '{}' participant '{participant}' is not present in compiled asset '{}'",
                    scene.id, scene.asset
                ),
                scene_key_span(file, source, input.scene_index, "participants"),
            ));
        }
    }

    for compiled_source in &input.asset.sources {
        match input.current_sources.get(compiled_source.path.as_str()) {
            Some(Some(current_source)) => {
                let current_fingerprint = canonical_source_fingerprint(current_source);
                if current_fingerprint != compiled_source.fingerprint {
                    diagnostics.push(diagnostic(
                        STALE_SOURCE_FINGERPRINT,
                        format!(
                            "compiled asset '{}' is stale for source '{}'",
                            scene.asset, compiled_source.path
                        ),
                        scene_key_span(file, source, input.scene_index, "asset"),
                    ));
                }
            }
            Some(None) | None => {
                diagnostics.push(diagnostic(
                    MISSING_SOURCE_ASSET,
                    format!(
                        "compiled asset '{}' references missing source '{}'",
                        scene.asset, compiled_source.path
                    ),
                    scene_key_span(file, source, input.scene_index, "asset"),
                ));
            }
        }
    }

    if input
        .current_schema_fingerprint
        .is_some_and(|current| input.asset.header.schema_fingerprint != current)
    {
        diagnostics.push(diagnostic(
            STALE_SCHEMA_FINGERPRINT,
            format!(
                "compiled asset '{}' has a stale schema fingerprint",
                scene.asset
            ),
            scene_key_span(file, source, input.scene_index, "asset"),
        ));
    }

    if input.asset.header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 {
        diagnostics.push(diagnostic(
            STALE_COMPILER_COMPATIBILITY,
            format!(
                "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                scene.asset,
                input.asset.header.compiler_compatibility_version,
                COMPILER_COMPATIBILITY_VERSION_V0
            ),
            scene_key_span(file, source, input.scene_index, "asset"),
        ));
    }

    if input.asset.header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0 {
        diagnostics.push(diagnostic(
            MALFORMED_COMPILED_ASSET,
            format!(
                "compiled asset '{}' uses unsupported format version {}",
                scene.asset, input.asset.header.format_version
            ),
            scene_key_span(file, source, input.scene_index, "asset"),
        ));
    }

    diagnostics
}
