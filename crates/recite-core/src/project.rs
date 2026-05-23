use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledDialogue,
    Diagnostic, DiagnosticCode, DiagnosticSeverity, ProjectSchema, SchemaFingerprint,
    SourcePosition, SourceSpan, canonical_source_fingerprint,
};

pub const MALFORMED_MANIFEST: &str = "RECITE_PROJECT001";
pub const DUPLICATE_SCENE_ID: &str = "RECITE_PROJECT002";
pub const MISSING_COMPILED_ASSET: &str = "RECITE_PROJECT003";
pub const UNKNOWN_START_BLOCK: &str = "RECITE_PROJECT004";
pub const MISSING_PARTICIPANTS: &str = "RECITE_PROJECT005";
pub const MISSING_SOURCE_ASSET: &str = "RECITE_PROJECT006";
pub const MALFORMED_COMPILED_ASSET: &str = "RECITE_PROJECT007";
pub const UNKNOWN_PARTICIPANT: &str = "RECITE_PROJECT008";

pub const STALE_SOURCE_FINGERPRINT: &str = "RECITE_FRESH001";
pub const STALE_SCHEMA_FINGERPRINT: &str = "RECITE_FRESH002";
pub const STALE_COMPILER_COMPATIBILITY: &str = "RECITE_FRESH003";

/// Loaded `recite.project.toml` manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifest {
    pub project: ProjectManifestMetadata,
    pub scenes: Vec<ProjectScene>,
}

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
                diagnostics: vec![diagnostic(
                    MALFORMED_MANIFEST,
                    format!("malformed project manifest: {error}"),
                    toml_error_span(&file, source, &error),
                )],
            },
        }
    }
}

/// Result of loading a project manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestLoadReport {
    pub manifest: Option<ProjectManifest>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Top-level project manifest metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectManifestMetadata {
    pub content_set: Option<String>,
    pub version: Option<String>,
    pub schema: Option<String>,
}

/// One scene entry in `recite.project.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectScene {
    pub id: String,
    pub presentation: Option<String>,
    pub asset: String,
    pub block: String,
    pub participants: Vec<String>,
    pub cinematic_scene: Option<String>,
}

/// Filesystem and decoded-asset data needed for freshness validation.
pub struct ProjectFreshnessInput<'a> {
    pub scene_index: usize,
    pub scene: &'a ProjectScene,
    pub asset: &'a CompiledDialogue,
    pub current_sources: BTreeMap<&'a str, Option<&'a str>>,
    pub current_schema_fingerprint: Option<SchemaFingerprint>,
}

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
            diagnostics.push(diagnostic(
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
                    diagnostics.push(diagnostic(
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

#[must_use]
pub fn project_scene_key_span(
    file: &str,
    source: &str,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    scene_key_span(file, source, scene_index, key)
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
                diagnostic(
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

fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("project diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}

fn toml_error_span(file: &str, source: &str, error: &toml::de::Error) -> SourceSpan {
    let Some(span) = error.span() else {
        return manifest_span(file);
    };
    let start = byte_offset_position(source, span.start).unwrap_or_else(point_one);
    SourceSpan::point(file.to_owned(), start)
}

fn scene_key_span(file: &str, source: &str, scene_index: usize, key: &str) -> SourceSpan {
    scene_key_position(source, scene_index, key)
        .or_else(|| scene_header_position(source, scene_index))
        .map_or_else(
            || manifest_span(file),
            |position| SourceSpan::point(file, position),
        )
}

fn manifest_span(file: &str) -> SourceSpan {
    SourceSpan::point(file.to_owned(), point_one())
}

fn scene_key_position(source: &str, scene_index: usize, key: &str) -> Option<SourcePosition> {
    let mut current_scene = None;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[scenes]]") {
            current_scene = Some(current_scene.map_or(0, |index| index + 1));
            continue;
        }

        if current_scene == Some(scene_index) && trimmed.starts_with(key) {
            let column = line.find(key).unwrap_or(0) + 1;
            return source_position(line_index + 1, column);
        }
    }

    None
}

fn scene_header_position(source: &str, scene_index: usize) -> Option<SourcePosition> {
    let mut current_scene = 0;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[scenes]]") {
            if current_scene == scene_index {
                let column = line.find("[[scenes]]").unwrap_or(0) + 1;
                return source_position(line_index + 1, column);
            }
            current_scene += 1;
        }
    }

    None
}

fn byte_offset_position(source: &str, offset: usize) -> Option<SourcePosition> {
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, character) in source.char_indices() {
        if index >= offset {
            return source_position(line, column);
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    source_position(line, column)
}

fn point_one() -> SourcePosition {
    SourcePosition::new(1, 1).expect("1-based position is valid")
}

fn source_position(line: usize, column: usize) -> Option<SourcePosition> {
    let line = u32::try_from(line).ok()?;
    let column = u32::try_from(column).ok()?;
    SourcePosition::new(line, column).ok()
}
