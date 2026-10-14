mod assessment;
pub use assessment::ProjectAssetFreshness;
use assessment::SourceIssue;

use super::{
    MISSING_SOURCE_ASSET, ProjectFreshnessInput, ProjectManifestSource,
    STALE_COMPILER_COMPATIBILITY, STALE_SCHEMA_FINGERPRINT, STALE_SOURCE_FINGERPRINT,
    UNKNOWN_PARTICIPANT, UNKNOWN_START_BLOCK, UNSUPPORTED_ASSET_VERSION,
    diagnostics::project_diagnostic, spans::scene_key_span,
};
use crate::{
    Diagnostic, DiagnosticArgumentValue, compiled::COMPILED_ASSET_FORMAT_VERSION_V0,
    compiled::COMPILER_COMPATIBILITY_VERSION_V0, toml_spans::TomlSpanIndex,
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
    let assessment = ProjectAssetFreshness::new(
        input.asset,
        &input.current_sources,
        input.current_schema_fingerprint,
    );
    validate_scene(
        file,
        source,
        spans,
        input.scene_index,
        input.scene,
        &assessment,
    )
}

fn validate_scene(
    file: &str,
    source: &str,
    spans: Option<&TomlSpanIndex>,
    scene_index: usize,
    scene: &super::ProjectScene,
    assessment: &ProjectAssetFreshness<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if assessment
        .asset
        .block_lookup
        .as_slice()
        .binary_search_by(|entry| entry.id.as_str().cmp(scene.block.as_str()))
        .is_err()
    {
        diagnostics.push(project_diagnostic(
            &UNKNOWN_START_BLOCK,
            "diagnostic-project-004",
            format!(
                "scene '{}' references unknown block '{}'",
                scene.id, scene.block
            ),
            scene_key_span(file, source, spans, scene_index, "block"),
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

    for participant in &scene.participants {
        if !assessment.speakers.is_empty() && !assessment.speakers.contains(participant.as_str()) {
            diagnostics.push(project_diagnostic(
                &UNKNOWN_PARTICIPANT,
                "diagnostic-project-008-compiled-asset",
                format!(
                    "scene '{}' participant '{participant}' is not present in compiled asset '{}'",
                    scene.id, scene.asset
                ),
                scene_key_span(file, source, spans, scene_index, "participants"),
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

    for (compiled_source, issue) in &assessment.source_issues {
        match issue {
            SourceIssue::Stale => {
                diagnostics.push(project_diagnostic(
                    &STALE_SOURCE_FINGERPRINT,
                    "diagnostic-fresh-001",
                    format!(
                        "compiled asset '{}' is stale for source '{}'",
                        scene.asset, compiled_source.path
                    ),
                    scene_key_span(file, source, spans, scene_index, "asset"),
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
            SourceIssue::Missing => {
                diagnostics.push(project_diagnostic(
                    &MISSING_SOURCE_ASSET,
                    "diagnostic-project-006",
                    format!(
                        "compiled asset '{}' references missing source '{}'",
                        scene.asset, compiled_source.path
                    ),
                    scene_key_span(file, source, spans, scene_index, "asset"),
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

    if assessment.schema_stale {
        diagnostics.push(project_diagnostic(
            &STALE_SCHEMA_FINGERPRINT,
            "diagnostic-fresh-002",
            format!(
                "compiled asset '{}' has a stale schema fingerprint",
                scene.asset
            ),
            scene_key_span(file, source, spans, scene_index, "asset"),
            [(
                "asset",
                DiagnosticArgumentValue::String(scene.asset.clone()),
            )],
        ));
    }

    if assessment.asset.header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 {
        diagnostics.push(project_diagnostic(
            &STALE_COMPILER_COMPATIBILITY,
            "diagnostic-fresh-003",
            format!(
                "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                scene.asset,
                assessment.asset.header.compiler_compatibility_version,
                COMPILER_COMPATIBILITY_VERSION_V0
            ),
            scene_key_span(file, source, spans, scene_index, "asset"),
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String(scene.asset.clone()),
                ),
                (
                    "version",
                    DiagnosticArgumentValue::Integer(i64::from(
                        assessment.asset.header.compiler_compatibility_version,
                    )),
                ),
                (
                    "expected",
                    DiagnosticArgumentValue::Integer(i64::from(COMPILER_COMPATIBILITY_VERSION_V0)),
                ),
            ],
        ));
    }

    if assessment.asset.header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0 {
        diagnostics.push(project_diagnostic(
            &UNSUPPORTED_ASSET_VERSION,
            "diagnostic-project-007",
            format!(
                "compiled asset '{}' uses unsupported format version {}",
                scene.asset, assessment.asset.header.format_version
            ),
            scene_key_span(file, source, spans, scene_index, "asset"),
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String(scene.asset.clone()),
                ),
                (
                    "version",
                    DiagnosticArgumentValue::Integer(i64::from(
                        assessment.asset.header.format_version,
                    )),
                ),
            ],
        ));
    }

    diagnostics
}
