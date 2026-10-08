//! Saved-project builds shared by CLI watch and Writer.

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
pub use preparation::{classify_discovery_error, prepare_discovered};
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
