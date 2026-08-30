use std::path::Path;

use super::project_relative_key;

#[test]
fn project_relative_key_joins_utf8_components_with_slashes() {
    let project_root = Path::new("project");
    let path = project_root.join("nested").join("café.recite");

    assert_eq!(
        project_relative_key(project_root, &path).as_deref(),
        Some("nested/café.recite")
    );
}
