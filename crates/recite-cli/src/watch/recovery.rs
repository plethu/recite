use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use super::targets::TargetMapError;

/// A stage marker requiring explicit host cleanup or recovery inspection.
#[derive(Clone, Debug)]
#[non_exhaustive]
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

    pub(super) fn new(marker: PathBuf, reason: ProjectBuildRecoveryReason) -> Self {
        Self {
            marker,
            reason,
            detail: ProjectBuildRecoveryDetail::None,
        }
    }

    pub(super) fn with_io(
        marker: PathBuf,
        reason: ProjectBuildRecoveryReason,
        error: &io::Error,
    ) -> Self {
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
#[non_exhaustive]
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
#[non_exhaustive]
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

/// Encode a marker path without lossy Unicode conversion.
///
/// Unix paths are encoded as `u1~` followed by their raw bytes in hex.
/// Windows paths are encoded as `w1~` followed by UTF-16 code units in
/// hex. Both forms preserve separators, control bytes, and non-Unicode names
/// so the encoded marker cannot collide with a reason or list boundary. The
/// versioned prefixes contain no record delimiters.
pub(super) fn encode_marker_path(path: &Path) -> String {
    #[cfg(unix)]
    {
        encode_bytes("u1~", path.as_os_str().as_bytes())
    }
    #[cfg(windows)]
    {
        let mut encoded = String::from("w1~");
        for unit in path.as_os_str().encode_wide() {
            use std::fmt::Write;
            let _ = write!(encoded, "{unit:04x}");
        }
        encoded
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_str().map_or_else(
            || String::from("p1~nonunicode"),
            |value| {
                use std::fmt::Write;
                let mut encoded = String::from("p1~");
                for character in value.chars() {
                    let _ = write!(encoded, "{character:04x}");
                }
                encoded
            },
        )
    }
}

#[cfg(unix)]
fn encode_bytes(prefix: &str, bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut encoded = String::from(prefix);
    for byte in bytes {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
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

#[cfg(test)]
mod tests;
