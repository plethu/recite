mod diagnostics;
mod freshness;
mod manifest;
mod spans;
mod validate;

use std::collections::BTreeMap;

use crate::{CompiledDialogue, DiagnosticCode, SchemaFingerprint};

pub use freshness::{validate_project_freshness, validate_project_freshness_source};
pub use spans::project_scene_key_span;
pub use validate::{validate_project_manifest, validate_project_manifest_source};

pub const MALFORMED_MANIFEST: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT001");
pub const DUPLICATE_SCENE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT002");
pub const MISSING_COMPILED_ASSET: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT003");
pub const UNKNOWN_START_BLOCK: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT004");
pub const MISSING_PARTICIPANTS: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT005");
pub const MISSING_SOURCE_ASSET: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT006");
pub const MALFORMED_COMPILED_ASSET: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PROJECT007");
pub const UNSUPPORTED_ASSET_VERSION: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PROJECT007");
pub const UNKNOWN_PARTICIPANT: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROJECT008");

pub const STALE_SOURCE_FINGERPRINT: DiagnosticCode = DiagnosticCode::new_static("RECITE_FRESH001");
pub const STALE_SCHEMA_FINGERPRINT: DiagnosticCode = DiagnosticCode::new_static("RECITE_FRESH002");
pub const STALE_COMPILER_COMPATIBILITY: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_FRESH003");

/// Loaded `recite.project.toml` manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifest {
    /// The version of the project manifest syntax, when declared.
    pub format_version: Option<u32>,
    pub project: ProjectManifestMetadata,
    /// Filesystem discovery configuration for project-owned source files.
    pub discovery: ProjectDiscovery,
    pub scenes: Vec<ProjectScene>,
}

/// Project-owned source discovery settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDiscovery {
    /// Project-relative source roots in declaration order.
    pub source_roots: Vec<String>,
    /// Project-relative slash-separated exclusion globs.
    pub excludes: Vec<String>,
}

impl Default for ProjectDiscovery {
    fn default() -> Self {
        Self {
            source_roots: vec![".".to_owned()],
            excludes: Vec::new(),
        }
    }
}

/// Result of loading a project manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestLoadReport {
    pub manifest: Option<ProjectManifest>,
    pub diagnostics: Vec<crate::Diagnostic>,
}

/// A successfully parsed project manifest together with its source-backed
/// TOML ranges.
#[derive(Clone, Debug)]
pub struct ProjectManifestSource {
    pub(super) file: String,
    pub(super) source_text: String,
    pub(super) manifest: ProjectManifest,
    pub(super) spans: crate::toml_spans::TomlSpanIndex,
}

impl PartialEq for ProjectManifestSource {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
            && self.source_text == other.source_text
            && self.manifest == other.manifest
    }
}

impl Eq for ProjectManifestSource {}

/// Result of loading a source-backed project manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestSourceLoadReport {
    pub source: Option<ProjectManifestSource>,
    pub diagnostics: Vec<crate::Diagnostic>,
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
