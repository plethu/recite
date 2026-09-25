#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

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

#[cfg(test)]
mod tests;
