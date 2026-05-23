mod freshness;
mod manifest;
mod spans;
mod validate;

use std::collections::BTreeMap;

use crate::{CompiledDialogue, SchemaFingerprint};

pub use freshness::validate_project_freshness;
pub use spans::project_scene_key_span;
pub use validate::validate_project_manifest;

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

/// Result of loading a project manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestLoadReport {
    pub manifest: Option<ProjectManifest>,
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
