#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FileIdentity {
    Saved(SavedFileIdentity),
    Open(OpenFileIdentity),
}

impl FileIdentity {
    pub(crate) fn uri(&self) -> &Uri {
        match self {
            Self::Saved(identity) => &identity.uri,
            Self::Open(identity) => &identity.uri,
        }
    }

    pub(crate) fn saved_path(&self) -> Option<&Path> {
        match self {
            Self::Saved(identity) => Some(&identity.canonical_path),
            Self::Open(identity) => identity.saved_path.as_deref(),
        }
    }

    pub(crate) fn project_relative_path(&self) -> Option<&str> {
        match self {
            Self::Saved(identity) => Some(&identity.project_relative_path),
            Self::Open(identity) => identity.project_relative_path.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedFileIdentity {
    pub(crate) uri: Uri,
    pub(crate) canonical_path: PathBuf,
    pub(crate) project_relative_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenFileIdentity {
    pub(crate) uri: Uri,
    pub(crate) saved_path: Option<PathBuf>,
    pub(crate) project_relative_path: Option<String>,
}
use std::path::{Path, PathBuf};

use lsp_types::Uri;
