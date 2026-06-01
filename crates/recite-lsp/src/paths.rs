use std::path::{Component, Path, PathBuf};

use lsp_types::Uri;

pub(crate) fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    let rest = uri.as_str().strip_prefix("file://")?;
    if !rest.starts_with('/') {
        return None;
    }

    percent_decode(rest).map(PathBuf::from)
}

pub(crate) fn file_path_to_uri(path: &Path) -> Option<Uri> {
    let path = path.to_str()?;
    format!("file://{}", percent_encode_path(path))
        .parse::<Uri>()
        .ok()
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

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).copied()?;
            let low = bytes.get(index + 2).copied()?;
            decoded.push((hex_value(high)? << 4) | hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'.' | b'-' | b'_' | b'~' => {
                encoded.push(char::from(byte))
            }
            _ => {
                encoded.push('%');
                encoded.push(hex_digit(byte >> 4));
                encoded.push(hex_digit(byte & 0x0f));
            }
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'A' + (value - 10)),
        _ => '0',
    }
}
