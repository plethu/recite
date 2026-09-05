use std::path::Path;

use serde::Serialize;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(test)]
mod tests;

/// An exact, machine-readable representation of the lexical input path.
///
/// Display paths intentionally remain a separate compatibility surface for
/// human diagnostics. This representation does not replace separators or
/// apply lossy Unicode conversion, so distinct inputs remain distinct.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "encoding", content = "value", rename_all = "snake_case")]
pub(crate) enum MachinePathProjection {
    Utf8(String),
    #[cfg(unix)]
    UnixBytes(String),
    #[cfg(windows)]
    WindowsWtf16(Vec<u16>),
}

pub(crate) fn machine_path(path: &Path) -> MachinePathProjection {
    #[cfg(unix)]
    {
        let bytes = path.as_os_str().as_bytes();
        match std::str::from_utf8(bytes) {
            Ok(value) => MachinePathProjection::Utf8(value.to_owned()),
            Err(_) => MachinePathProjection::UnixBytes(hex_bytes(bytes)),
        }
    }

    #[cfg(windows)]
    {
        let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
        match String::from_utf16(&units) {
            Ok(value) => MachinePathProjection::Utf8(value),
            Err(_) => MachinePathProjection::WindowsWtf16(units),
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        MachinePathProjection::Utf8(path.to_string_lossy().into_owned())
    }
}

#[cfg(unix)]
fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}
