use std::path::Path;

use super::{MachinePathProjection, machine_path};

#[test]
fn literal_backslash_and_separator_remain_distinct() {
    let separator = machine_path(Path::new("schema/part.toml"));
    let literal_backslash = machine_path(Path::new(r"schema\part.toml"));

    assert_ne!(separator, literal_backslash);
    assert_eq!(
        separator,
        MachinePathProjection::Utf8("schema/part.toml".to_owned())
    );
    assert_eq!(
        literal_backslash,
        MachinePathProjection::Utf8(r"schema\part.toml".to_owned())
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_unix_path_uses_raw_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let path = std::ffi::OsString::from_vec(b"schema-\xff.toml".to_vec());
    assert_eq!(
        machine_path(Path::new(&path)),
        MachinePathProjection::UnixBytes("736368656d612dff2e746f6d6c".to_owned())
    );
}

#[cfg(windows)]
#[test]
fn windows_drive_and_unc_paths_remain_exact_text() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let drive = OsString::from_wide(&[b'C' as u16, b':' as u16, b'\\' as u16, b't' as u16]);
    let unc = OsString::from_wide(&[
        b'\\' as u16,
        b'\\' as u16,
        b's' as u16,
        b'e' as u16,
        b'r' as u16,
        b'v' as u16,
        b'e' as u16,
        b'r' as u16,
    ]);

    assert_eq!(
        machine_path(Path::new(&drive)),
        MachinePathProjection::Utf8(r"C:\t".to_owned())
    );
    assert_eq!(
        machine_path(Path::new(&unc)),
        MachinePathProjection::Utf8(r"\\server".to_owned())
    );
}

#[cfg(windows)]
#[test]
fn unpaired_windows_units_use_wtf16_representation() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let path = OsString::from_wide(&[0x0063, 0xd800, 0x0061]);
    assert_eq!(
        machine_path(Path::new(&path)),
        MachinePathProjection::WindowsWtf16(vec![0x0063, 0xd800, 0x0061])
    );
}
