use std::path::{Path, PathBuf};

use recite_compiler::{
    BuildGeneration, BuildRequest, BuildTarget, BuildTargetError, SnapshotGeneration,
};
use recite_core::{Diagnostic, ProjectManifestSource, ProjectSchema};

/// A project asset target selected from the manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProjectBuildTarget {
    pub(super) target: BuildTarget,
}

impl ProjectBuildTarget {
    pub(super) fn new(asset: &str) -> Result<Self, BuildTargetError> {
        Ok(Self {
            target: BuildTarget::new(asset.to_owned())?,
        })
    }

    /// The logical project-relative output target.
    #[must_use]
    pub const fn target(&self) -> &BuildTarget {
        &self.target
    }

    /// The compiled asset ID used for this target.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        self.target.as_str()
    }
}

/// A complete, validated project build request owned by the CLI boundary.
///
/// The embedded compiler request carries all content-bearing inputs. The
/// manifest and source inputs use project-relative slash
/// [`recite_core::DocumentKey`]s and
/// `Saved` authority, while a valid schema is carried as its canonical model.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProjectBuildRequest {
    pub(super) project_root: PathBuf,
    pub(super) manifest: ProjectManifestSource,
    pub(super) schema: Option<ProjectSchema>,
    pub(super) build: BuildRequest,
    pub(super) targets: Vec<ProjectBuildTarget>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl ProjectBuildRequest {
    /// Discover, validate, and prepare a saved project build.
    ///
    /// Content failures are returned as [`ProjectBuildPreparation::Rejected`]
    /// so callers retain every structured diagnostic without attempting to
    /// place malformed data in a compiler [`BuildRequest`]. Filesystem read
    /// failures remain typed preparation errors.
    pub fn prepare(
        project_root: impl AsRef<Path>,
    ) -> Result<ProjectBuildPreparation, ProjectBuildPreparationError> {
        Self::prepare_with_generations(
            project_root,
            BuildGeneration::initial(),
            SnapshotGeneration::initial(),
        )
    }

    /// Prepare a project request with explicit lifecycle generations.
    pub fn prepare_with_generations(
        project_root: impl AsRef<Path>,
        generation: BuildGeneration,
        snapshot_generation: SnapshotGeneration,
    ) -> Result<ProjectBuildPreparation, ProjectBuildPreparationError> {
        super::preparation::prepare(project_root.as_ref(), generation, snapshot_generation)
    }

    /// The canonical compiler request carried by this project request.
    #[must_use]
    pub const fn build_request(&self) -> &BuildRequest {
        &self.build
    }

    /// The discovered canonical project root.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The source-backed project manifest used during preparation.
    #[must_use]
    pub const fn manifest(&self) -> &ProjectManifestSource {
        &self.manifest
    }

    /// The canonical schema, when the project declares one.
    #[must_use]
    pub const fn schema(&self) -> Option<&ProjectSchema> {
        self.schema.as_ref()
    }

    /// Deterministically ordered unique manifest asset targets.
    #[must_use]
    pub fn targets(&self) -> &[ProjectBuildTarget] {
        &self.targets
    }

    /// Non-error preparation diagnostics, such as overlapping-root warnings.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Result of content preparation before a compiler request exists.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProjectBuildPreparation {
    Ready(Box<ProjectBuildRequest>),
    Rejected { diagnostics: Vec<Diagnostic> },
}

impl ProjectBuildPreparation {
    /// Return the prepared request, if all project content was valid.
    #[must_use]
    pub fn request(&self) -> Option<&ProjectBuildRequest> {
        match self {
            Self::Ready(request) => Some(request),
            Self::Rejected { .. } => None,
        }
    }

    /// Return structured diagnostics from discovery or content validation.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Ready(request) => request.diagnostics(),
            Self::Rejected { diagnostics } => diagnostics,
        }
    }

    /// Whether this outcome contains a usable compiler request.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Consume a ready outcome into its project request.
    #[must_use]
    pub fn into_request(self) -> Option<ProjectBuildRequest> {
        match self {
            Self::Ready(request) => Some(*request),
            Self::Rejected { .. } => None,
        }
    }
}

/// Errors that prevent a project request from being prepared at all.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectBuildPreparationError {
    #[error(transparent)]
    Discovery(#[from] recite_config::ProjectDiscoveryError),
    #[error("failed to read project input {path}: {message}")]
    Read { path: PathBuf, message: String },
    #[error("project contains no .recite inputs")]
    NoInputs,
    #[error("schema path {path} is not project-relative: {reason}")]
    InvalidSchemaPath { path: PathBuf, reason: String },
    #[error("schema path {declared} resolves outside the canonical project root to {resolved}")]
    SchemaOutsideProject {
        declared: PathBuf,
        resolved: PathBuf,
    },
    #[error("schema input {path} loaded without a canonical model")]
    SchemaWithoutModel { path: PathBuf },
    #[error("invalid project input key {key:?}: {reason}")]
    InvalidInputKey { key: String, reason: String },
    #[error("authoring validation could not be completed: {message}")]
    Authoring { message: String },
    #[error(transparent)]
    Request(#[from] recite_compiler::BuildRequestError),
    #[error(transparent)]
    Target(#[from] BuildTargetError),
}
