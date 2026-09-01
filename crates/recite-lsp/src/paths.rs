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

/// Produce a stable, valid document-key string for a path that has no shared
/// project-relative root. The URI form is preferred because it preserves
/// Windows drive and UNC identities; the component form remains available for
/// synthetic or non-URI-representable paths.
pub(crate) fn stable_path_identity(path: &Path) -> String {
    let identity = file_path_to_uri(path)
        .map(|uri| uri.as_str().as_bytes().to_vec())
        .unwrap_or_else(|| path_identity_bytes(path));
    let mut encoded = String::from("~lsp/");
    for byte in identity {
        use std::fmt::Write as _;

        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn path_identity_bytes(path: &Path) -> Vec<u8> {
    let mut identity = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                append_component(&mut identity, b"prefix", Some(prefix.as_os_str()));
            }
            Component::RootDir => append_component(&mut identity, b"root", None),
            Component::CurDir => append_component(&mut identity, b"current", None),
            Component::ParentDir => append_component(&mut identity, b"parent", None),
            Component::Normal(value) => {
                append_component(&mut identity, b"normal", Some(value));
            }
        }
    }
    identity
}

fn append_component(identity: &mut Vec<u8>, kind: &[u8], value: Option<&std::ffi::OsStr>) {
    identity.extend_from_slice(kind);
    identity.push(0);
    if let Some(value) = value {
        append_os_str(identity, value);
    }
    identity.push(0xff);
}

#[cfg(unix)]
fn append_os_str(identity: &mut Vec<u8>, value: &std::ffi::OsStr) {
    use std::os::unix::ffi::OsStrExt;

    identity.extend_from_slice(value.as_bytes());
}

#[cfg(windows)]
fn append_os_str(identity: &mut Vec<u8>, value: &std::ffi::OsStr) {
    use std::os::windows::ffi::OsStrExt;

    for unit in value.encode_wide() {
        identity.extend_from_slice(&unit.to_le_bytes());
    }
}

#[cfg(not(any(unix, windows)))]
fn append_os_str(identity: &mut Vec<u8>, value: &std::ffi::OsStr) {
    identity.extend_from_slice(value.to_string_lossy().as_bytes());
}

fn component_text(component: Component<'_>) -> Option<String> {
    match component {
        Component::Normal(value) => value.to_str().map(str::to_owned),
        Component::CurDir => Some(".".to_owned()),
        Component::ParentDir => Some("..".to_owned()),
        Component::RootDir | Component::Prefix(_) => None,
    }
}
