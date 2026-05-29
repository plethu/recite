use serde::Deserialize;

use super::{
    MALFORMED_MANIFEST, ProjectManifest, ProjectManifestLoadReport, ProjectManifestMetadata,
    ProjectScene,
};
use crate::{Diagnostic, project::spans::toml_error_span};

impl ProjectManifest {
    #[must_use]
    pub fn load_str(file: impl Into<String>, source: &str) -> ProjectManifestLoadReport {
        let file = file.into();
        match toml::from_str::<RawProjectManifest>(source) {
            Ok(raw) => ProjectManifestLoadReport {
                manifest: Some(raw.into_manifest()),
                diagnostics: Vec::new(),
            },
            Err(error) => ProjectManifestLoadReport {
                manifest: None,
                diagnostics: vec![Diagnostic::error(
                    MALFORMED_MANIFEST,
                    format!("malformed project manifest: {error}"),
                    toml_error_span(&file, source, &error),
                )],
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectManifest {
    #[serde(default)]
    project: RawProjectMetadata,
    #[serde(default)]
    scenes: Vec<RawProjectScene>,
}

impl RawProjectManifest {
    fn into_manifest(self) -> ProjectManifest {
        ProjectManifest {
            project: ProjectManifestMetadata {
                content_set: self.project.content_set,
                version: self.project.version,
                schema: self.project.schema,
            },
            scenes: self
                .scenes
                .into_iter()
                .map(RawProjectScene::into_scene)
                .collect(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectMetadata {
    content_set: Option<String>,
    version: Option<String>,
    schema: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectScene {
    id: String,
    presentation: Option<String>,
    asset: String,
    block: String,
    #[serde(default)]
    participants: Vec<String>,
    cinematic_scene: Option<String>,
}

impl RawProjectScene {
    fn into_scene(self) -> ProjectScene {
        ProjectScene {
            id: self.id,
            presentation: self.presentation,
            asset: self.asset,
            block: self.block,
            participants: self.participants,
            cinematic_scene: self.cinematic_scene,
        }
    }
}
