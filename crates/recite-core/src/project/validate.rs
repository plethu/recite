use std::collections::BTreeMap;

use super::{
    DUPLICATE_SCENE_ID, MISSING_PARTICIPANTS, ProjectManifest, UNKNOWN_PARTICIPANT,
    spans::scene_key_span,
};
use crate::{Diagnostic, ProjectSchema};

/// Validate manifest-only project policy.
#[must_use]
pub fn validate_project_manifest(
    file: &str,
    source: &str,
    manifest: &ProjectManifest,
    schema: Option<&ProjectSchema>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(validate_duplicate_scene_ids(file, source, manifest));

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        if scene.participants.is_empty() {
            diagnostics.push(Diagnostic::error(
                MISSING_PARTICIPANTS,
                format!("scene '{}' must declare at least one participant", scene.id),
                scene_key_span(file, source, scene_index, "participants"),
            ));
        }

        if let Some(schema) = schema
            && !schema.speakers.is_empty()
        {
            for participant in &scene.participants {
                if !schema.speakers.contains_key(participant) {
                    diagnostics.push(Diagnostic::error(
                        UNKNOWN_PARTICIPANT,
                        format!(
                            "scene '{}' references unknown participant '{participant}'",
                            scene.id
                        ),
                        scene_key_span(file, source, scene_index, "participants"),
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
) -> Vec<Diagnostic> {
    let mut seen = BTreeMap::<&str, usize>::new();
    let mut diagnostics = Vec::new();

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        if let Some(first_index) = seen.get(scene.id.as_str()).copied() {
            diagnostics.push(
                Diagnostic::error(
                    DUPLICATE_SCENE_ID,
                    format!("duplicate scene id '{}'", scene.id),
                    scene_key_span(file, source, scene_index, "id"),
                )
                .with_related([crate::RelatedSpan::new(
                    scene_key_span(file, source, first_index, "id"),
                    "first scene with this id",
                )]),
            );
        } else {
            seen.insert(&scene.id, scene_index);
        }
    }

    diagnostics
}
