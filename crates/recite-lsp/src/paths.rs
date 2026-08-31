use std::path::{Component, Path, PathBuf};

use lsp_types::Uri;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileUriError {
    InvalidUri,
    NotFileUri,
    InvalidFilePath,
    InvalidLspUri,
}

pub(crate) fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    uri_to_file_path_checked(uri).ok()
}

pub(crate) fn file_path_to_uri(path: &Path) -> Option<Uri> {
    file_path_to_uri_checked(path).ok()
}

pub(crate) fn uri_to_file_path_checked(uri: &Uri) -> Result<PathBuf, FileUriError> {
    let parsed = Url::parse(uri.as_str()).map_err(|_| FileUriError::InvalidUri)?;
    if parsed.scheme() != "file" {
        return Err(FileUriError::NotFileUri);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() || parsed.port().is_some() {
        return Err(FileUriError::InvalidFilePath);
    }
    parsed
        .to_file_path()
        .map_err(|()| FileUriError::InvalidFilePath)
}

pub(crate) fn file_path_to_uri_checked(path: &Path) -> Result<Uri, FileUriError> {
    let url = Url::from_file_path(path).map_err(|()| FileUriError::InvalidFilePath)?;
    url.as_str()
        .parse()
        .map_err(|_| FileUriError::InvalidLspUri)
}

pub(crate) fn project_relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let parts = relative
        .components()
        .filter_map(component_text)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn component_text(component: Component<'_>) -> Option<String> {
    match component {
        Component::Normal(value) => value.to_str().map(str::to_owned),
        Component::CurDir => Some(".".to_owned()),
        Component::ParentDir => Some("..".to_owned()),
        Component::RootDir | Component::Prefix(_) => None,
    }
}
