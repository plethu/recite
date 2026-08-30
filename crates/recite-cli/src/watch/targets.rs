use std::fs;
use std::path::{Component, Path, PathBuf};

use recite_compiler::{BuildInputKind, BuildTarget};

use super::request::ProjectBuildRequest;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TargetPathError {
    #[error("target path is absolute")]
    Absolute,
    #[error("target path contains a parent component")]
    Parent,
    #[error("target path contains an empty or current component")]
    EmptyOrCurrent,
    #[error("target path contains a backslash or platform prefix")]
    PlatformAmbiguous,
    #[error("target path resolves outside the project root")]
    OutsideProject,
    #[error("target path names a directory")]
    Directory,
}

#[derive(Debug)]
pub(super) struct TargetMap {
    pub(super) root: PathBuf,
    pub(super) first_target: BuildTarget,
    pub(super) targets: std::collections::BTreeMap<BuildTarget, PathBuf>,
}

impl TargetMap {
    pub(super) fn from_request(request: &ProjectBuildRequest) -> Result<Self, TargetMapError> {
        let root = fs::canonicalize(request.project_root()).map_err(|source| {
            TargetMapError::ProjectRoot {
                path: request.project_root().to_owned(),
                message: source.to_string(),
            }
        })?;
        let input_paths = request
            .build_request()
            .inputs()
            .iter()
            .filter(|input| {
                matches!(
                    input.kind(),
                    BuildInputKind::Manifest | BuildInputKind::Schema | BuildInputKind::Source
                )
            })
            .map(|input| root.join(input.key().as_str()))
            .collect::<Vec<_>>();
        let mut targets = std::collections::BTreeMap::new();
        for project_target in request.targets() {
            let target = project_target.target().clone();
            let relative = validate_target(target.as_str()).map_err(|reason| {
                TargetMapError::InvalidTarget {
                    target: target.clone(),
                    reason,
                }
            })?;
            let output = root.join(relative);
            ensure_contained(&root, &output).map_err(|reason| TargetMapError::InvalidTarget {
                target: target.clone(),
                reason,
            })?;
            if output.exists() && fs::metadata(&output).is_ok_and(|metadata| metadata.is_dir()) {
                return Err(TargetMapError::InvalidTarget {
                    target,
                    reason: TargetPathError::Directory,
                });
            }
            if let Some(output_canonical) = canonical_existing_path(&output) {
                ensure_contained(&root, &output_canonical).map_err(|reason| {
                    TargetMapError::InvalidTarget {
                        target: target.clone(),
                        reason,
                    }
                })?;
                for input in &input_paths {
                    if fs::canonicalize(input)
                        .ok()
                        .is_some_and(|input| input == output_canonical)
                    {
                        return Err(TargetMapError::AliasesInput {
                            target,
                            input: input.clone(),
                        });
                    }
                }
            }
            targets.insert(target, output);
        }
        if targets.is_empty() {
            return Err(TargetMapError::NoTargets);
        }
        let first_target = targets
            .keys()
            .next()
            .cloned()
            .ok_or(TargetMapError::NoTargets)?;
        Ok(Self {
            root,
            first_target,
            targets,
        })
    }
}

fn validate_target(value: &str) -> Result<PathBuf, TargetPathError> {
    if value.contains('\\')
        || value.starts_with("//")
        || (value.len() >= 2
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':')
    {
        return Err(TargetPathError::PlatformAmbiguous);
    }
    if value
        .split('/')
        .any(|component| component.is_empty() || component == ".")
    {
        return Err(TargetPathError::EmptyOrCurrent);
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(TargetPathError::Absolute);
    }
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => relative.push(component),
            Component::ParentDir => return Err(TargetPathError::Parent),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {
                return Err(TargetPathError::EmptyOrCurrent);
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(TargetPathError::EmptyOrCurrent);
    }
    Ok(relative)
}

fn ensure_contained(root: &Path, path: &Path) -> Result<(), TargetPathError> {
    path.strip_prefix(root)
        .map(|_| ())
        .map_err(|_| TargetPathError::OutsideProject)
}

fn canonical_existing_path(path: &Path) -> Option<PathBuf> {
    if fs::symlink_metadata(path)
        .ok()
        .is_some_and(|metadata| metadata.file_type().is_symlink())
    {
        return fs::canonicalize(path).ok();
    }
    let mut existing = path.to_owned();
    loop {
        if existing.exists() {
            return fs::canonicalize(existing).ok();
        }
        if !existing.pop() {
            return None;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TargetMapError {
    #[error("project manifest contains no output targets")]
    NoTargets,
    #[error("failed to resolve project root {path}: {message}")]
    ProjectRoot { path: PathBuf, message: String },
    #[error("invalid project target {target}: {reason}")]
    InvalidTarget {
        target: BuildTarget,
        reason: TargetPathError,
    },
    #[error("project target {target} aliases source input {input}")]
    AliasesInput { target: BuildTarget, input: PathBuf },
}
