use std::collections::BTreeSet;

use super::{
    MISSING_SOURCE_ASSET, ProjectFreshnessInput, ProjectManifestSource,
    STALE_COMPILER_COMPATIBILITY, STALE_SCHEMA_FINGERPRINT, STALE_SOURCE_FINGERPRINT,
    UNKNOWN_PARTICIPANT, UNKNOWN_START_BLOCK, UNSUPPORTED_ASSET_VERSION,
    diagnostics::project_diagnostic, spans::scene_key_span,
};
use crate::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, Diagnostic,
    DiagnosticArgumentValue, canonical_source_fingerprint, toml_spans::TomlSpanIndex,
};

/// Validate one decoded asset against the project scene and current source/schema state.
#[must_use]
pub fn validate_project_freshness(
    file: &str,
    source: &str,
    input: ProjectFreshnessInput<'_>,
) -> Vec<Diagnostic> {
    let spans = toml_edit::Document::parse(source.to_owned())
        .ok()
        .map(|document| TomlSpanIndex::from_document(&document));
    validate_project_freshness_with_spans(file, source, input, spans.as_ref())
}

/// Validate freshness from a parsed project source without reparsing its TOML.
#[must_use]
pub fn validate_project_freshness_source(
    source: &ProjectManifestSource,
    input: ProjectFreshnessInput<'_>,
) -> Vec<Diagnostic> {
    validate_project_freshness_with_spans(
        source.file(),
        source.source_text_ref(),
        input,
        Some(source.spans_ref()),
    )
}

fn validate_project_freshness_with_spans(
    file: &str,
    source: &str,
    input: ProjectFreshnessInput<'_>,
    spans: Option<&TomlSpanIndex>,
) -> Vec<Diagnostic> {
    let scene = input.scene;
    let mut diagnostics = Vec::new();

    if !input
        .asset
        .block_lookup
        .iter()
        .any(|entry| entry.id.as_str() == scene.block)
    {
        diagnostics.push(project_diagnostic(
            &UNKNOWN_START_BLOCK,
            "diagnostic-project-004",
            format!(
                "scene '{}' references unknown block '{}'",
                scene.id, scene.block
            ),
            scene_key_span(file, source, spans, input.scene_index, "block"),
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String(scene.id.clone()),
                ),
                (
                    "block",
                    DiagnosticArgumentValue::String(scene.block.clone()),
                ),
            ],
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
            diagnostics.push(project_diagnostic(
                &UNKNOWN_PARTICIPANT,
                "diagnostic-project-008-compiled-asset",
                format!(
                    "scene '{}' participant '{participant}' is not present in compiled asset '{}'",
                    scene.id, scene.asset
                ),
                scene_key_span(file, source, spans, input.scene_index, "participants"),
                [
                    (
                        "scene_id",
                        DiagnosticArgumentValue::String(scene.id.clone()),
                    ),
                    (
                        "participant",
                        DiagnosticArgumentValue::String(participant.clone()),
                    ),
                    (
                        "asset",
                        DiagnosticArgumentValue::String(scene.asset.clone()),
                    ),
                ],
            ));
        }
    }

    for compiled_source in &input.asset.sources {
        match input.current_sources.get(compiled_source.path.as_str()) {
            Some(Some(current_source)) => {
                let current_fingerprint = canonical_source_fingerprint(current_source);
                if current_fingerprint != compiled_source.fingerprint {
                    diagnostics.push(project_diagnostic(
                        &STALE_SOURCE_FINGERPRINT,
                        "diagnostic-fresh-001",
                        format!(
                            "compiled asset '{}' is stale for source '{}'",
                            scene.asset, compiled_source.path
                        ),
                        scene_key_span(file, source, spans, input.scene_index, "asset"),
                        [
                            (
                                "asset",
                                DiagnosticArgumentValue::String(scene.asset.clone()),
                            ),
                            (
                                "source",
                                DiagnosticArgumentValue::String(compiled_source.path.clone()),
                            ),
                        ],
                    ));
                }
            }
            Some(None) | None => {
                diagnostics.push(project_diagnostic(
                    &MISSING_SOURCE_ASSET,
                    "diagnostic-project-006",
                    format!(
                        "compiled asset '{}' references missing source '{}'",
                        scene.asset, compiled_source.path
                    ),
                    scene_key_span(file, source, spans, input.scene_index, "asset"),
                    [
                        (
                            "asset",
                            DiagnosticArgumentValue::String(scene.asset.clone()),
                        ),
                        (
                            "source",
                            DiagnosticArgumentValue::String(compiled_source.path.clone()),
                        ),
                    ],
                ));
            }
        }
    }

    if input
        .current_schema_fingerprint
        .is_some_and(|current| input.asset.header.schema_fingerprint != current)
    {
        diagnostics.push(project_diagnostic(
            &STALE_SCHEMA_FINGERPRINT,
            "diagnostic-fresh-002",
            format!(
                "compiled asset '{}' has a stale schema fingerprint",
                scene.asset
            ),
            scene_key_span(file, source, spans, input.scene_index, "asset"),
            [(
                "asset",
                DiagnosticArgumentValue::String(scene.asset.clone()),
            )],
        ));
    }

    if input.asset.header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 {
        diagnostics.push(project_diagnostic(
            &STALE_COMPILER_COMPATIBILITY,
            "diagnostic-fresh-003",
            format!(
                "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                scene.asset,
                input.asset.header.compiler_compatibility_version,
                COMPILER_COMPATIBILITY_VERSION_V0
            ),
            scene_key_span(file, source, spans, input.scene_index, "asset"),
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String(scene.asset.clone()),
                ),
                (
                    "version",
                    DiagnosticArgumentValue::Integer(i64::from(
                        input.asset.header.compiler_compatibility_version,
                    )),
                ),
                (
                    "expected",
                    DiagnosticArgumentValue::Integer(i64::from(COMPILER_COMPATIBILITY_VERSION_V0)),
                ),
            ],
        ));
    }

    if input.asset.header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0 {
        diagnostics.push(project_diagnostic(
            &UNSUPPORTED_ASSET_VERSION,
            "diagnostic-project-007",
            format!(
                "compiled asset '{}' uses unsupported format version {}",
                scene.asset, input.asset.header.format_version
            ),
            scene_key_span(file, source, spans, input.scene_index, "asset"),
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String(scene.asset.clone()),
                ),
                (
                    "version",
                    DiagnosticArgumentValue::Integer(i64::from(input.asset.header.format_version)),
                ),
            ],
        ));
    }

    diagnostics
}
