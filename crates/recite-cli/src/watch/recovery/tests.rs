use std::path::PathBuf;

use super::encode_marker_path;

#[cfg(unix)]
#[test]
fn marker_encoding_preserves_arbitrary_unix_bytes_and_boundaries() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let marker = PathBuf::from(OsString::from_vec(b"stage/\xff\n.tmp".to_vec()));
    let encoded = encode_marker_path(&marker);
    assert_eq!(encoded, "u1~73746167652fff0a2e746d70");
    assert!(
        !encoded
            .chars()
            .any(|character| matches!(character, ':' | ';' | '\r' | '\n' | '\t'))
    );
    assert_ne!(
        encode_marker_path(&PathBuf::from("stage/ff.tmp")),
        encode_marker_path(&marker)
    );
}

#[cfg(windows)]
#[test]
fn marker_encoding_preserves_windows_wide_path_units() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let drive_units = [0x0043, 0x003a, 0x005c, 0xd800, 0x000a, 0x0058];
    let drive = PathBuf::from(OsString::from_wide(&drive_units));
    let encoded = encode_marker_path(&drive);
    assert_eq!(encoded, "w1~0043003a005cd800000a0058");
    assert!(
        !encoded
            .chars()
            .any(|character| matches!(character, ':' | ';' | '\r' | '\n' | '\t'))
    );
    let round_trip = encoded
        .strip_prefix("w1~")
        .expect("versioned prefix")
        .as_bytes()
        .chunks_exact(4)
        .map(|unit| u16::from_str_radix(std::str::from_utf8(unit).expect("hex"), 16).expect("unit"))
        .collect::<Vec<_>>();
    assert_eq!(
        OsString::from_wide(&round_trip).as_os_str(),
        drive.as_os_str()
    );

    let unc = PathBuf::from(r"\\server\share\Σ.recite-stage");
    assert_eq!(
        encode_marker_path(&unc),
        "w1~005c005c007300650072007600650072005c00730068006100720065005c03a3002e007200650063006900740065002d0073007400610067006500"
    );
}
