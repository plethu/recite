//! The dialogue sources one command reads, keyed the way the project keys them.
//!
//! Inside a project, every source is keyed by its project-relative slash path (§11.1), so a
//! cross-file target such as `src/objects.recite::objects` resolves the same way whether the author
//! passes `src`, `.`, an absolute path or a single file, and from any working directory. Validation
//! also reads the project's other sources as context and reports only the named files. Outside a
//! project, keys keep each command's established form. Diagnostics are moved back from keys to
//! paths relative to the working directory before anyone sees them.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use recite_compiler::compile::CompileInput;
use recite_config::{ProjectDiscoveryReport, discover_project};
use recite_core::Diagnostic;

use super::inputs::{collect_input_files, common_path_prefix, compile_path_root};
use super::paths::display_path;
use crate::error::{CliError, ProjectDefaultError};

/// How sources are keyed when no project contains them.
#[derive(Clone, Copy)]
pub(crate) enum LooseKeys<'a> {
    /// As the author typed the path.
    AsTyped,
    /// Relative to the directory shared by the inputs and the compiled output.
    OutputRoot(&'a Path),
    /// Relative to the directory the inputs share.
    CommonDirectory,
}

pub(crate) struct SourceSet {
    /// The sources the command acts on, in path order.
    pub(crate) named: Vec<KeyedSource>,
    /// Other project sources, read only so references into them resolve.
    pub(crate) context: Vec<KeyedSource>,
    project: Option<ProjectDiscoveryReport>,
    display: BTreeMap<String, String>,
}

pub(crate) struct KeyedSource {
    pub(crate) path: PathBuf,
    pub(crate) key: String,
    pub(crate) source: String,
}

impl KeyedSource {
    pub(crate) fn compile_input(&self) -> CompileInput {
        CompileInput::new(self.key.clone(), self.source.clone())
    }
}

impl SourceSet {
    /// Read the named paths, or with none the sources of the project at the working directory.
    pub(crate) fn read(paths: &[PathBuf], loose: LooseKeys<'_>) -> Result<Self, CliError> {
        if paths.is_empty() {
            return Self::project_sources();
        }
        let files = collect_input_files(paths)?;
        let canonical = canonical_paths(&files)?;
        if let Some((project, root)) = containing_project(&files[0], &canonical) {
            return Self::in_project(files, &canonical, project, &root);
        }
        let keys = loose_keys(&files, &canonical, loose);
        let named = read_keyed(files.into_iter().zip(keys))?;
        Ok(Self::new(named, Vec::new(), None))
    }

    fn project_sources() -> Result<Self, CliError> {
        let start = std::env::current_dir().map_err(CliError::Io)?;
        let project = match discover_project(&start) {
            Ok(project) => project,
            Err(recite_config::ProjectDiscoveryError::NotFound { .. }) => {
                return Err(ProjectDefaultError::NoInputsOutsideProject.into());
            }
            Err(source) => return Err(CliError::ProjectDiscovery { source }),
        };
        let named = project
            .documents()
            .iter()
            .map(|document| KeyedSource {
                path: document.path().to_owned(),
                key: document.key().as_str().to_owned(),
                source: document.text().to_owned(),
            })
            .collect::<Vec<_>>();
        if named.is_empty() {
            return Err(CliError::NoInputs);
        }
        Ok(Self::new(named, Vec::new(), Some(project)))
    }

    fn in_project(
        files: Vec<PathBuf>,
        canonical: &[PathBuf],
        project: ProjectDiscoveryReport,
        root: &Path,
    ) -> Result<Self, CliError> {
        let keys = canonical
            .iter()
            .map(|path| project_key(root, path))
            .collect::<Vec<_>>();
        let named = read_keyed(files.into_iter().zip(keys))?;
        let named_keys = named
            .iter()
            .map(|source| source.key.as_str())
            .collect::<BTreeSet<_>>();
        let context = project
            .documents()
            .iter()
            .filter(|document| !named_keys.contains(document.key().as_str()))
            .map(|document| KeyedSource {
                path: document.path().to_owned(),
                key: document.key().as_str().to_owned(),
                source: document.text().to_owned(),
            })
            .collect();
        Ok(Self::new(named, context, Some(project)))
    }

    fn new(
        named: Vec<KeyedSource>,
        context: Vec<KeyedSource>,
        project: Option<ProjectDiscoveryReport>,
    ) -> Self {
        let display = named
            .iter()
            .chain(&context)
            .map(|source| (source.key.clone(), shown_path(&source.path)))
            .collect();
        Self {
            named,
            context,
            project,
            display,
        }
    }

    /// The schema the project declares, as a path to read.
    pub(crate) fn declared_schema(&self) -> Option<PathBuf> {
        let manifest = self.project.as_ref()?.manifest();
        let key = manifest.schema_key().ok()??;
        Some(manifest.project_root().join(key.as_str()))
    }

    /// The schema a command checks against: the one passed, or else the project's. The second
    /// value is the project's schema when an explicit one differs from it, for a warning.
    pub(crate) fn effective_schema(&self, explicit: Option<&Path>) -> SchemaChoice {
        let declared = self.declared_schema();
        match (explicit, declared) {
            (Some(explicit), Some(declared)) => SchemaChoice {
                differs_from: (!same_file(explicit, &declared)).then_some(declared),
                path: Some(explicit.to_owned()),
            },
            (Some(explicit), None) => SchemaChoice {
                path: Some(explicit.to_owned()),
                differs_from: None,
            },
            (None, declared) => SchemaChoice {
                path: declared,
                differs_from: None,
            },
        }
    }

    pub(crate) fn named_paths(&self) -> Vec<PathBuf> {
        self.named
            .iter()
            .map(|source| source.path.clone())
            .collect()
    }

    pub(crate) fn named_inputs(&self) -> Vec<CompileInput> {
        self.named.iter().map(KeyedSource::compile_input).collect()
    }

    pub(crate) fn is_named(&self, key: &str) -> bool {
        self.named.iter().any(|source| source.key == key)
    }

    /// Replace source keys in diagnostics with the paths the author can open from here.
    pub(crate) fn relocate(&self, diagnostics: &mut [Diagnostic]) {
        let shown = |file: &mut String| {
            if let Some(path) = self.display.get(file.as_str()) {
                file.clone_from(path);
            }
        };
        for diagnostic in diagnostics {
            shown(&mut diagnostic.span.file);
            for related in &mut diagnostic.related {
                shown(&mut related.span.file);
            }
            for related in &mut diagnostic.related_presentations {
                shown(&mut related.span.file);
            }
        }
    }
}

pub(crate) struct SchemaChoice {
    pub(crate) path: Option<PathBuf>,
    /// The project's declared schema, when the explicit one is a different file.
    pub(crate) differs_from: Option<PathBuf>,
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn canonical_paths(files: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    files
        .iter()
        .map(|path| {
            fs::canonicalize(path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })
        })
        .collect()
}

/// The nearest project above the first source, when every source lies inside it. A malformed
/// manifest is left to the project commands; plain source commands keep working without it.
fn containing_project(
    first: &Path,
    canonical: &[PathBuf],
) -> Option<(ProjectDiscoveryReport, PathBuf)> {
    let start = if first.is_dir() {
        first
    } else {
        first.parent().unwrap_or(first)
    };
    let start = if start.as_os_str().is_empty() {
        Path::new(".")
    } else {
        start
    };
    let project = discover_project(start).ok()?;
    let root = fs::canonicalize(project.manifest().project_root()).ok()?;
    canonical
        .iter()
        .all(|path| path.starts_with(&root))
        .then_some((project, root))
}

fn project_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| display_path(path), display_path)
}

fn loose_keys(files: &[PathBuf], canonical: &[PathBuf], loose: LooseKeys<'_>) -> Vec<String> {
    let root = match loose {
        LooseKeys::AsTyped => None,
        LooseKeys::OutputRoot(output) => compile_path_root(output, files),
        LooseKeys::CommonDirectory => canonical
            .iter()
            .filter_map(|path| path.parent().map(Path::to_owned))
            .reduce(|left, right| common_path_prefix(&left, &right).unwrap_or_default()),
    };
    files
        .iter()
        .zip(canonical)
        .map(|(file, canonical)| {
            root.as_deref()
                .and_then(|root| canonical.strip_prefix(root).ok())
                .map(display_path)
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| display_path(file))
        })
        .collect()
}

fn read_keyed(
    files: impl IntoIterator<Item = (PathBuf, String)>,
) -> Result<Vec<KeyedSource>, CliError> {
    files
        .into_iter()
        .map(|(path, key)| {
            let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            Ok(KeyedSource { path, key, source })
        })
        .collect()
}

/// A path relative to the working directory when it lies below it, without a `./` prefix.
fn shown_path(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    std::env::current_dir()
        .ok()
        .and_then(|cwd| fs::canonicalize(cwd).ok())
        .and_then(|cwd| canonical.strip_prefix(cwd).ok().map(Path::to_owned))
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || display_path(path.strip_prefix(".").unwrap_or(path)),
            |relative| display_path(&relative),
        )
}
