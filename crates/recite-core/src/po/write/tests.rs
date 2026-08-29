use std::path::Path;

#[test]
fn bare_relative_paths_use_current_directory_as_parent() {
    assert_eq!(
        super::normalized_parent(Path::new("catalog.po")),
        Path::new(".")
    );
}
