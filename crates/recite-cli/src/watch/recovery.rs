use std::path::{Path, PathBuf};

use super::targets::TargetMapError;

/// A stage marker requiring explicit host cleanup or recovery inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProjectBuildRecovery {
    marker: PathBuf,
    message: String,
}

impl ProjectBuildRecovery {
    #[must_use]
    pub fn marker(&self) -> &Path {
        &self.marker
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(super) fn new(marker: PathBuf, message: String) -> Self {
        Self { marker, message }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectBuildPublisherError {
    #[error(transparent)]
    Targets(#[from] TargetMapError),
}
