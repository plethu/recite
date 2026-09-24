use std::io;
use std::path::{Path, PathBuf};

use super::targets::TargetMapError;

/// A stage marker requiring explicit host cleanup or recovery inspection.
#[derive(Clone, Debug)]
pub struct ProjectBuildRecovery {
    marker: PathBuf,
    reason: ProjectBuildRecoveryReason,
    detail: ProjectBuildRecoveryDetail,
}

impl PartialEq for ProjectBuildRecovery {
    fn eq(&self, other: &Self) -> bool {
        self.marker == other.marker && self.reason == other.reason
    }
}

impl Eq for ProjectBuildRecovery {}

impl Ord for ProjectBuildRecovery {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.marker
            .cmp(&other.marker)
            .then_with(|| self.reason.cmp(&other.reason))
    }
}

impl PartialOrd for ProjectBuildRecovery {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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

    #[must_use]
    pub fn detail(&self) -> ProjectBuildRecoveryDetail {
        self.detail.clone()
    }

    pub fn new(marker: PathBuf, reason: ProjectBuildRecoveryReason) -> Self {
        Self {
            marker,
            reason,
            detail: ProjectBuildRecoveryDetail::None,
        }
    }

    pub fn with_io(marker: PathBuf, reason: ProjectBuildRecoveryReason, error: &io::Error) -> Self {
        Self {
            marker,
            reason,
            detail: ProjectBuildRecoveryDetail::Io {
                kind: ProjectBuildRecoveryIoKind::from_error(error),
                raw_os_error: error.raw_os_error(),
                message: error.to_string(),
            },
        }
    }
}

/// Stable, structured detail for a recovery record.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectBuildRecoveryDetail {
    None,
    Io {
        kind: ProjectBuildRecoveryIoKind,
        raw_os_error: Option<i32>,
        message: String,
    },
}

/// Portable category for the I/O cause that left a recovery marker.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectBuildRecoveryIoKind {
    AlreadyExists,
    InvalidInput,
    NotFound,
    PermissionDenied,
    Other,
}

impl ProjectBuildRecoveryIoKind {
    fn from_error(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            io::ErrorKind::InvalidInput => Self::InvalidInput,
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Other,
        }
    }
}

/// The structured reason a publisher left a stage marker for host recovery.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectBuildRecoveryReason {
    StageCleanupFailed,
    PublicationIndeterminate,
    PublicationUncommitted,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProjectBuildPublisherError {
    #[error(transparent)]
    Targets(#[from] TargetMapError),
}
