//! The smallest project `recite validate-project` accepts: a manifest and one scene with a single
//! default block and one frozen line. Writer's "New project…" and the CLI test share this.
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_core::{SourceId, SourceIdKind};

use super::{PROJECT_MANIFEST_FILE, PROJECT_MANIFEST_FORMAT_VERSION};

/// Why a starter project was not written.
#[derive(Debug, thiserror::Error)]
pub enum StarterProjectError {
    #[error("{} already contains a {PROJECT_MANIFEST_FILE}; open it instead", folder.display())]
    AlreadyAProject { folder: PathBuf },
    #[error("{} already exists; choose another project name", path.display())]
    SceneExists { path: PathBuf },
    #[error("could not write {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// A project name as a file stem and content set: lowercase ASCII words joined by `_`.
#[must_use]
pub fn starter_slug(name: &str) -> String {
    let slug = name
        .trim()
        .to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if slug.is_empty() {
        "story".to_owned()
    } else {
        slug
    }
}

/// The manifest and scene source for a new project named `name`.
#[must_use]
pub fn starter_files(name: &str) -> [(String, String); 2] {
    let slug = starter_slug(name);
    let scene = format!("{slug}.recite");
    let anchor =
        SourceId::generated_anchor(&scene, SourceIdKind::Line, 2, 1, "opening", 0).to_string();
    [
        (
            PROJECT_MANIFEST_FILE.to_owned(),
            format!(
                "format_version = {PROJECT_MANIFEST_FORMAT_VERSION}\n\n[project]\ncontent_set = \"{slug}\"\nversion = \"0.1.0\"\n"
            ),
        ),
        (
            scene,
            format!(":: start default\n> opening@{anchor}\n  This is where the scene begins.\n"),
        ),
    ]
}

/// Create `folder` if needed and write a starter project into it. Returns the scene path.
///
/// # Errors
/// Refuses a folder that already holds a project manifest or the starter scene; reports I/O
/// failures with the path that failed. Existing files are never overwritten.
pub fn create_starter_project(folder: &Path, name: &str) -> Result<PathBuf, StarterProjectError> {
    if folder.join(PROJECT_MANIFEST_FILE).exists() {
        return Err(StarterProjectError::AlreadyAProject {
            folder: folder.to_owned(),
        });
    }
    let files = starter_files(name);
    let scene = folder.join(&files[1].0);
    if scene.exists() {
        return Err(StarterProjectError::SceneExists { path: scene });
    }
    std::fs::create_dir_all(folder).map_err(|source| StarterProjectError::Io {
        path: folder.to_owned(),
        source,
    })?;
    for (relative, contents) in &files {
        let path = folder.join(relative);
        let io = |source| StarterProjectError::Io {
            path: path.clone(),
            source,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(io)?;
        file.write_all(contents.as_bytes()).map_err(io)?;
    }
    Ok(scene)
}
