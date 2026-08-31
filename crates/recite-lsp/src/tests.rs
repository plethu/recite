#[path = "tests/authoring_registry/tests.rs"]
mod authoring_registry;
mod availability;
mod code_action;
#[path = "tests/config_registry/tests.rs"]
mod config_registry;
mod diagnostics;
mod edit_projection;
mod lifecycle;
mod navigation;
mod navigation_corrections;
mod navigation_ranges;
mod position;
mod project_indexes;
#[path = "tests/protocol_registry/tests.rs"]
mod protocol_registry;
mod support;
mod sync;

mod paths {
    use std::path::{Path, PathBuf};

    use lsp_types::Uri;

    use crate::paths::{FileUriError, file_path_to_uri_checked, uri_to_file_path_checked};

    fn uri(value: &str) -> Uri {
        value
            .parse()
            .unwrap_or_else(|error| panic!("valid test URI {value}: {error}"))
    }

    #[cfg(unix)]
    #[test]
    fn unix_file_paths_round_trip_percent_and_space() {
        let path = PathBuf::from("/tmp/recite %/schema.toml");
        let uri =
            file_path_to_uri_checked(&path).unwrap_or_else(|error| panic!("file URI: {error:?}"));

        assert_eq!(uri.as_str(), "file:///tmp/recite%20%25/schema.toml");
        assert_eq!(uri_to_file_path_checked(&uri), Ok(path));
    }

    #[cfg(unix)]
    #[test]
    fn lexical_alias_uri_round_trips_without_normalizing_ownership_text() {
        let alias = uri("file:///tmp/recite%20%25/./schema.toml");

        assert_eq!(
            uri_to_file_path_checked(&alias),
            Ok(PathBuf::from("/tmp/recite %/./schema.toml"))
        );
    }

    #[test]
    fn windows_drive_and_unc_uri_shapes_are_stable() {
        let drive = uri("file:///C:/Users/Recite%20User/schema.toml");
        let unc = uri("file://server/share/Recite%20User/schema.toml");

        assert_eq!(
            url::Url::parse(drive.as_str())
                .unwrap_or_else(|error| panic!("drive URL: {error}"))
                .path(),
            "/C:/Users/Recite%20User/schema.toml"
        );
        assert_eq!(
            url::Url::parse(unc.as_str())
                .unwrap_or_else(|error| panic!("UNC URL: {error}"))
                .host_str(),
            Some("server")
        );

        #[cfg(windows)]
        {
            let drive_path = PathBuf::from(r"C:\Users\Recite User\schema.toml");
            let unc_path = PathBuf::from(r"\\server\share\Recite User\schema.toml");

            assert_eq!(uri_to_file_path_checked(&drive), Ok(drive_path.clone()));
            assert_eq!(uri_to_file_path_checked(&unc), Ok(unc_path.clone()));
            assert_eq!(file_path_to_uri_checked(&drive_path), Ok(drive));
            assert_eq!(file_path_to_uri_checked(&unc_path), Ok(unc));
        }
    }

    #[test]
    fn non_file_and_invalid_file_uris_are_rejected_without_panics() {
        assert_eq!(
            uri_to_file_path_checked(&uri("https://example.test/schema.toml")),
            Err(FileUriError::NotFileUri)
        );
        assert_eq!(
            uri_to_file_path_checked(&uri("file://localhost:8080/schema.toml")),
            Err(FileUriError::InvalidUri)
        );
        assert_eq!(
            uri_to_file_path_checked(&uri("file://user@example.test/schema.toml")),
            Err(FileUriError::InvalidUri)
        );
        assert_eq!(
            file_path_to_uri_checked(Path::new("relative/schema.toml")),
            Err(FileUriError::InvalidFilePath)
        );
    }
}
