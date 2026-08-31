use std::path::{Path, PathBuf};

use super::targets::TargetMapError;

/// A stage marker requiring explicit host cleanup or recovery inspection.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct ProjectBuildRecovery {
    marker: PathBuf,
    reason: ProjectBuildRecoveryReason,
}

impl ProjectBuildRecovery {
    #[must_use]
    pub fn marker(&self) -> &Path {
        &self.marker
    }

    #[must_use]
    pub fn reason(&self) -> ProjectBuildRecoveryReason {
        self.reason
    }

    pub(super) fn new(marker: PathBuf, reason: ProjectBuildRecoveryReason) -> Self {
        Self { marker, reason }
    }
}

/// The structured reason a publisher left a stage marker for host recovery.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ProjectBuildRecoveryReason {
    StageCleanupFailed,
    PublicationIndeterminate,
    PublicationUncommitted,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectBuildPublisherError {
    #[error(transparent)]
    Targets(#[from] TargetMapError),
}
