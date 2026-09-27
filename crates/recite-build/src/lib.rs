//! Saved-project builds shared by CLI watch and Writer.

use std::path::{Path, PathBuf};

use recite_core::schema::SchemaLoadReport;

mod commit;
mod engine;
mod preparation;
mod publisher;
mod recovery;
mod request;
mod staging;
mod target_identity;
mod targets;

pub use engine::ProjectBuildEngine;
pub use preparation::{classify_discovery_error, prepare_discovered, schema_document_key};
pub use publisher::{ProjectBuildPublisher, ProjectPreparedBuild};
pub use recovery::{
    ProjectBuildPublisherError, ProjectBuildRecovery, ProjectBuildRecoveryDetail,
    ProjectBuildRecoveryIoKind, ProjectBuildRecoveryReason,
};
pub use request::{
    ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest, ProjectBuildTarget,
};
pub use targets::{TargetMapError, TargetPathError};

const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";

fn resolve_project_path(project_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_owned()
    } else {
        project_root.join(path)
    }
}

fn load_schema(path: &Path) -> std::io::Result<SchemaLoadReport> {
    let source = std::fs::read_to_string(path)?;
    Ok(recite_core::schema::load_schema_manifest_str(
        path.to_string_lossy().replace('\\', "/"),
        &source,
    ))
}
