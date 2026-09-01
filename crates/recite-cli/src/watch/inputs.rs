use std::path::{Component, Path};

pub(super) fn is_project_recite_source(project_root: &Path, path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "recite")
        && path.strip_prefix(project_root).is_ok_and(|relative| {
            relative.components().all(|component| match component {
                Component::Normal(name) => {
                    let name = name.to_string_lossy();
                    !name.starts_with('.') && name != "target"
                }
                _ => true,
            })
        })
}

pub(super) fn is_generated_output_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "recitec")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.') && name.ends_with(".tmp"))
}
