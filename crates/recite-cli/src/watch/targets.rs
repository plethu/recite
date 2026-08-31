use std::fs;
use std::path::{Component, Path, PathBuf};

use recite_compiler::{BuildInputKind, BuildTarget};

use super::request::ProjectBuildRequest;
use super::target_identity::{PhysicalIdentity, physical_identity, same_physical_path};

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
    #[error("target path contains a non-directory component")]
    NonDirectoryComponent,
    #[error("target path contains a symlink component")]
    SymlinkComponent,
    #[error("could not inspect target path: {0}")]
    Inspection(String),
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
        let mut physical_targets: Vec<(PhysicalIdentity, BuildTarget)> = Vec::new();
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
            reject_symlink_components(&root, &output).map_err(|reason| {
                TargetMapError::InvalidTarget {
                    target: target.clone(),
                    reason,
                }
            })?;
            if output.exists() && fs::metadata(&output).is_ok_and(|metadata| metadata.is_dir()) {
                return Err(TargetMapError::InvalidTarget {
                    target,
                    reason: TargetPathError::Directory,
                });
            }
            let physical =
                physical_identity(&output).map_err(|message| TargetMapError::InvalidTarget {
                    target: target.clone(),
                    reason: TargetPathError::Inspection(message),
                })?;
            reject_symlink_components(&root, &output).map_err(|reason| {
                TargetMapError::InvalidTarget {
                    target: target.clone(),
                    reason,
                }
            })?;
            if let Some((_, existing)) = physical_targets
                .iter()
                .find(|(identity, _)| same_physical_path(identity, &physical))
            {
                return Err(TargetMapError::DuplicateDestination {
                    target,
                    existing: existing.clone(),
                    path: physical.canonical.clone(),
                });
            }
            for input in &input_paths {
                if physical_identity(input)
                    .ok()
                    .is_some_and(|input| same_physical_path(&input, &physical))
                {
                    return Err(TargetMapError::AliasesInput {
                        target,
                        input: input.clone(),
                    });
                }
            }
            physical_targets.push((physical, target.clone()));
            targets.insert(target, output);
        }
        let first_target = targets.keys().next().cloned().or_else(|| {
            // This value is used only to classify impossible publisher failures;
            // the coordinator handles an empty candidate set before publishing.
            BuildTarget::new("empty-project").ok()
        });
        let Some(first_target) = first_target else {
            return Err(TargetMapError::NoTargets);
        };
        Ok(Self {
            root,
            first_target,
            targets,
        })
    }
}

fn validate_target(value: &str) -> Result<PathBuf, TargetPathError> {
    let path = Path::new(value);
    let has_drive_prefix = value.len() >= 2
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':';
    if value.contains('\\') || value.starts_with("//") || has_drive_prefix {
        return Err(TargetPathError::PlatformAmbiguous);
    }
    if value.starts_with('/') {
        return Err(TargetPathError::Absolute);
    }
    #[cfg(not(windows))]
    if path.is_absolute() {
        return Err(TargetPathError::Absolute);
    }
    #[cfg(windows)]
    if path.is_absolute() && !has_drive_prefix {
        return Err(TargetPathError::Absolute);
    }
    if value
        .split('/')
        .any(|component| component.is_empty() || component == ".")
    {
        return Err(TargetPathError::EmptyOrCurrent);
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

pub(super) fn reject_symlink_components(root: &Path, output: &Path) -> Result<(), TargetPathError> {
    let relative = output
        .strip_prefix(root)
        .map_err(|_| TargetPathError::OutsideProject)?;
    let components = relative.components().collect::<Vec<_>>();
    let mut current = root.to_owned();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(TargetPathError::SymlinkComponent);
            }
            Ok(metadata) if index + 1 < components.len() && !metadata.is_dir() => {
                return Err(TargetPathError::NonDirectoryComponent);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(TargetPathError::Inspection(error.to_string())),
        }
    }
    Ok(())
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
    #[error("project targets {target} and {existing} resolve to the same destination {path}")]
    DuplicateDestination {
        target: BuildTarget,
        existing: BuildTarget,
        path: PathBuf,
    },
}
