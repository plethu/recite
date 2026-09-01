use std::ops::Range;

use serde::Deserialize;
use toml_edit::Document;

use super::{
    MALFORMED_MANIFEST, ProjectDiscovery, ProjectManifest, ProjectManifestLoadReport,
    ProjectManifestMetadata, ProjectManifestSource, ProjectManifestSourceLoadReport, ProjectScene,
};
use crate::{
    DiagnosticArgumentValue,
    project::{diagnostics::project_diagnostic, spans::toml_error_span},
    toml_spans::TomlSpanIndex,
};

impl ProjectManifest {
    #[must_use]
    pub fn load_str(file: impl Into<String>, source: &str) -> ProjectManifestLoadReport {
        let file = file.into();
        let document = match parse_document(source) {
            Ok(document) => document,
            Err((detail, range)) => {
                return ProjectManifestLoadReport {
                    manifest: None,
                    diagnostics: vec![malformed_diagnostic(&file, source, detail, range)],
                };
            }
        };
        match toml_edit::de::from_document::<RawProjectManifest>(document) {
            Ok(raw) => ProjectManifestLoadReport {
                manifest: Some(raw.into_manifest()),
                diagnostics: Vec::new(),
            },
            Err(error) => ProjectManifestLoadReport {
                manifest: None,
                diagnostics: vec![malformed_diagnostic(
                    &file,
                    source,
                    error.to_string(),
                    error.span(),
                )],
            },
        }
    }

    /// Parse a project manifest once and retain source-backed TOML ranges for
    /// validation and freshness diagnostics.
    #[must_use]
    pub fn load_str_with_spans(
        file: impl Into<String>,
        source: &str,
    ) -> ProjectManifestSourceLoadReport {
        let file = file.into();
        let document = match parse_document(source) {
            Ok(document) => document,
            Err((detail, range)) => {
                return ProjectManifestSourceLoadReport {
                    source: None,
                    diagnostics: vec![malformed_diagnostic(&file, source, detail, range)],
                };
            }
        };
        let spans = TomlSpanIndex::from_document(&document);
        let raw = match toml_edit::de::from_document::<RawProjectManifest>(document) {
            Ok(raw) => raw,
            Err(error) => {
                return ProjectManifestSourceLoadReport {
                    source: None,
                    diagnostics: vec![malformed_diagnostic(
                        &file,
                        source,
                        error.to_string(),
                        error.span(),
                    )],
                };
            }
        };

        ProjectManifestSourceLoadReport {
            source: Some(ProjectManifestSource {
                file,
                source_text: source.to_owned(),
                manifest: raw.into_manifest(),
                spans,
            }),
            diagnostics: Vec::new(),
        }
    }
}

fn parse_document(source: &str) -> Result<Document<String>, (String, Option<Range<usize>>)> {
    Document::parse(source.to_owned()).map_err(|error| (error.to_string(), error.span()))
}

fn malformed_diagnostic(
    file: &str,
    source: &str,
    detail: String,
    range: Option<Range<usize>>,
) -> crate::Diagnostic {
    project_diagnostic(
        &MALFORMED_MANIFEST,
        "diagnostic-project-001",
        format!("malformed project manifest: {detail}"),
        toml_error_span(file, source, range),
        [("detail", DiagnosticArgumentValue::String(detail))],
    )
}

impl ProjectManifestSource {
    /// The source path used for diagnostics.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Return the exact TOML text supplied to the loader.
    #[must_use]
    pub fn source_text(&self) -> String {
        self.source_text.clone()
    }

    pub(crate) fn source_text_ref(&self) -> &str {
        &self.source_text
    }

    pub(crate) fn spans_ref(&self) -> &TomlSpanIndex {
        &self.spans
    }

    /// Borrow the decoded project manifest.
    #[must_use]
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    /// Resolve a scene field through its decoded TOML path.
    #[must_use]
    pub fn scene_key_span(&self, scene_index: usize, key: &str) -> crate::SourceSpan {
        super::spans::scene_key_span_with_index(
            &self.file,
            &self.source_text,
            &self.spans,
            scene_index,
            key,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectManifest {
    format_version: Option<u32>,
    #[serde(default)]
    project: RawProjectMetadata,
    #[serde(default)]
    discovery: RawProjectDiscovery,
    #[serde(default)]
    scenes: Vec<RawProjectScene>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectDiscovery {
    #[serde(default)]
    source_roots: Option<Vec<String>>,
    #[serde(default)]
    excludes: Vec<String>,
}

impl RawProjectManifest {
    fn into_manifest(self) -> ProjectManifest {
        ProjectManifest {
            format_version: self.format_version,
            project: ProjectManifestMetadata {
                content_set: self.project.content_set,
                version: self.project.version,
                schema: self.project.schema,
            },
            discovery: ProjectDiscovery {
                source_roots: self
                    .discovery
                    .source_roots
                    .unwrap_or_else(|| ProjectDiscovery::default().source_roots),
                excludes: self.discovery.excludes,
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
