use std::collections::BTreeMap;

use super::{
    DUPLICATE_SCENE_ID, MISSING_PARTICIPANTS, ProjectManifest, ProjectManifestSource,
    UNKNOWN_PARTICIPANT,
    diagnostics::{project_diagnostic, related_presentation},
    spans::scene_key_span,
};
use crate::{Diagnostic, DiagnosticArgumentValue, ProjectSchema, toml_spans::TomlSpanIndex};

/// Validate manifest-only project policy.
#[must_use]
pub fn validate_project_manifest(
    file: &str,
    source: &str,
    manifest: &ProjectManifest,
    schema: Option<&ProjectSchema>,
) -> Vec<Diagnostic> {
    let spans = toml_edit::Document::parse(source.to_owned())
        .ok()
        .map(|document| TomlSpanIndex::from_document(&document));
    validate_project_manifest_with_spans(file, source, manifest, schema, spans.as_ref())
}

/// Validate a source-backed project manifest without reparsing its TOML.
#[must_use]
pub fn validate_project_manifest_source(
    source: &ProjectManifestSource,
    schema: Option<&ProjectSchema>,
) -> Vec<Diagnostic> {
    validate_project_manifest_with_spans(
        source.file(),
        source.source_text_ref(),
        source.manifest(),
        schema,
        Some(source.spans_ref()),
    )
}

fn validate_project_manifest_with_spans(
    file: &str,
    source: &str,
    manifest: &ProjectManifest,
    schema: Option<&ProjectSchema>,
    spans: Option<&TomlSpanIndex>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(validate_duplicate_scene_ids(file, source, manifest, spans));

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        if scene.participants.is_empty() {
            diagnostics.push(project_diagnostic(
                &MISSING_PARTICIPANTS,
                "diagnostic-project-005",
                format!("scene '{}' must declare at least one participant", scene.id),
                scene_key_span(file, source, spans, scene_index, "participants"),
                [(
                    "scene_id",
                    DiagnosticArgumentValue::String(scene.id.clone()),
                )],
            ));
        }

        if let Some(schema) = schema
            && !schema.speakers.is_empty()
        {
            for participant in &scene.participants {
                if !schema.speakers.contains_key(participant) {
                    diagnostics.push(project_diagnostic(
                        &UNKNOWN_PARTICIPANT,
                        "diagnostic-project-008",
                        format!(
                            "scene '{}' references unknown participant '{participant}'",
                            scene.id
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
                        ],
                    ));
                }
            }
        }
    }

    diagnostics
}

fn validate_duplicate_scene_ids(
    file: &str,
    source: &str,
    manifest: &ProjectManifest,
    spans: Option<&TomlSpanIndex>,
) -> Vec<Diagnostic> {
    let mut seen = BTreeMap::<&str, usize>::new();
    let mut diagnostics = Vec::new();

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        if let Some(first_index) = seen.get(scene.id.as_str()).copied() {
            diagnostics.push(
                project_diagnostic(
                    &DUPLICATE_SCENE_ID,
                    "diagnostic-project-002",
                    format!("duplicate scene id '{}'", scene.id),
                    scene_key_span(file, source, spans, scene_index, "id"),
                    [(
                        "scene_id",
                        DiagnosticArgumentValue::String(scene.id.clone()),
                    )],
                )
                .with_related_presentations([related_presentation(
                    scene_key_span(file, source, spans, first_index, "id"),
                    "diagnostic-project-002-related",
                )]),
            );
        } else {
            seen.insert(&scene.id, scene_index);
        }
    }

    diagnostics
}
