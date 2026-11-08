//! `id rename` and `id reassign`: deliberate stable-ID changes.
//!
//! Planning belongs to the shared authoring kernel, so the CLI rewrites the
//! header and every catalogue context exactly as an editor rename would. The
//! whole project namespace and every catalogue participate: an ID can only be
//! proven unused against all of them.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::authoring::{
    AuthoringEditError, AuthoringEditOperation, AuthoringKernel, AuthoringRequest, SavedDocument,
    StableIdRefusal, StableIdTarget,
};
use recite_config::{ProjectDiscoveryError, discover_project};
use recite_core::{DocumentKey, StableId, StableIdGenerator};

use crate::args::IdProjectArgs;
use crate::diagnostics::report_diagnostics;
use crate::error::{CliError, StableIdEditError};
use crate::fs::{collect_input_files, display_path, write_staged};
use crate::i18n::{Messages, MsgId};
use recite_ui::UiArg;

/// How the replacement ID is chosen.
pub(super) enum NewId<'a> {
    Named(&'a str),
    Generated,
}

pub(super) fn run(
    target: &str,
    new_id: NewId<'_>,
    args: &IdProjectArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let project = Project::load(args, stderr, messages)?;
    let mut kernel = AuthoringKernel::new();
    let generation = kernel.snapshot().generation();
    kernel
        .apply(
            AuthoringRequest::new(generation, project.saved(&project.sources), [])
                .with_catalogs(project.saved(&project.catalogs), []),
        )
        .map_err(|_| CliError::Diagnostics)?;
    let snapshot = kernel.snapshot();

    let target = project.target(target)?;
    let planned = match new_id {
        NewId::Named(id) => snapshot.plan_rename_stable_id(&target, id),
        NewId::Generated => {
            snapshot.plan_reassign_stable_id(&target, &mut StableIdGenerator::from_system())
        }
    };
    let plan = match planned {
        Ok(plan) => plan,
        Err(error) => return Err(project.refusal(error, &target, stderr, messages, snapshot)?),
    };

    let mut catalog_edits = 0_usize;
    for edit in plan.edits() {
        if project.catalogs.contains_key(edit.document()) {
            catalog_edits += 1;
        }
    }
    for (key, text) in plan.apply(snapshot).map_err(|_| CliError::Diagnostics)? {
        if let Some(path) = project.path(&key) {
            write_staged(path, text.as_bytes())?;
        }
    }
    match plan.operation() {
        AuthoringEditOperation::ReassignDuplicateStableId { from, to } => writeln!(
            stderr,
            "{}",
            messages.format(
                MsgId::CliIdsReassignedDuplicate,
                [("from", from.to_string()), ("to", to.to_string())]
            )
        )?,
        AuthoringEditOperation::RenameStableId { from, to } => {
            writeln!(
                stderr,
                "{}",
                messages.format(
                    MsgId::CliIdsRenamed,
                    [
                        ("from", UiArg::from(from.to_string())),
                        ("to", UiArg::from(to.to_string())),
                        ("count", UiArg::from(catalog_edits)),
                    ]
                )
            )?;
            if project.catalogs.is_empty() {
                writeln!(stderr, "{}", messages.text(MsgId::CliIdsNoCatalogs))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Every source and catalogue the edit reads, keyed relative to their
/// common directory so keys stay valid and unique.
struct Project {
    sources: BTreeMap<DocumentKey, (PathBuf, String)>,
    catalogs: BTreeMap<DocumentKey, (PathBuf, String)>,
}

impl Project {
    fn load(
        args: &IdProjectArgs,
        stderr: &mut dyn Write,
        messages: &Messages,
    ) -> Result<Self, CliError> {
        let start = args
            .paths
            .first()
            .cloned()
            .map_or_else(std::env::current_dir, Ok)
            .map_err(CliError::Io)?;
        let report = match discover_project(&start) {
            Ok(report) => Some(report),
            Err(ProjectDiscoveryError::NotFound { .. }) if !args.paths.is_empty() => None,
            Err(source) => return Err(CliError::ProjectDiscovery { source }),
        };
        // A partial walk could hide a header or catalogue using the new ID.
        if let Some(report) = &report
            && !report.is_complete()
        {
            let diagnostics = report
                .diagnostics()
                .iter()
                .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                .collect::<Vec<_>>();
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Err(CliError::Diagnostics);
        }

        let source_paths = if args.paths.is_empty() {
            report
                .iter()
                .flat_map(|report| report.documents())
                .map(|document| document.path().to_owned())
                .collect()
        } else {
            collect_input_files(&args.paths)?
        };
        if source_paths.is_empty() {
            return Err(CliError::NoInputs);
        }
        let mut catalog_paths = report
            .iter()
            .flat_map(|report| report.catalogs())
            .map(|catalog| catalog.path().to_owned())
            .collect::<Vec<_>>();
        catalog_paths.extend(args.catalogs.iter().cloned());

        let sources = canonical(source_paths)?;
        let catalogs = canonical(catalog_paths)?;
        let root = common_directory(sources.iter().chain(&catalogs));
        Ok(Self {
            sources: keyed(&root, sources)?,
            catalogs: keyed(&root, catalogs)?,
        })
    }

    fn saved(&self, documents: &BTreeMap<DocumentKey, (PathBuf, String)>) -> Vec<SavedDocument> {
        documents
            .iter()
            .map(|(key, (_, text))| SavedDocument::new(key.clone(), text.clone()))
            .collect()
    }

    fn path(&self, key: &DocumentKey) -> Option<&Path> {
        self.sources
            .get(key)
            .or_else(|| self.catalogs.get(key))
            .map(|(path, _)| path.as_path())
    }

    fn display(&self, key: &DocumentKey) -> String {
        self.path(key)
            .map_or_else(|| key.as_str().to_owned(), display_path)
    }

    /// `PATH:LINE` selects a header; anything else names a stable ID.
    fn target(&self, spelled: &str) -> Result<StableIdTarget, CliError> {
        let target_error = || {
            CliError::StableIdEdit(StableIdEditError::Target {
                target: spelled.to_owned(),
            })
        };
        if let Some((path, line)) = spelled.rsplit_once(':') {
            let line = line.parse::<u32>().map_err(|_| target_error())?;
            let path = std::fs::canonicalize(path).map_err(|_| target_error())?;
            let key = self
                .sources
                .iter()
                .find(|(_, (candidate, _))| *candidate == path)
                .map(|(key, _)| key.clone())
                .ok_or_else(target_error)?;
            return Ok(StableIdTarget::line(key, line));
        }
        let id = spelled
            .strip_prefix(recite_core::STABLE_ID_FIELD_PREFIX)
            .unwrap_or(spelled);
        StableId::new(id)
            .map(StableIdTarget::Id)
            .map_err(|_| CliError::StableIdEdit(StableIdEditError::Malformed { id: id.to_owned() }))
    }

    /// Maps a planning refusal to a structured CLI error. Source problems
    /// that need an author decision are reported as diagnostics instead.
    fn refusal(
        &self,
        error: AuthoringEditError,
        target: &StableIdTarget,
        stderr: &mut dyn Write,
        messages: &Messages,
        snapshot: &recite_compiler::authoring::AuthoringSnapshot,
    ) -> Result<CliError, CliError> {
        let refusal = match error {
            AuthoringEditError::InvalidStableId {
                id,
                reason: StableIdRefusal::ReservedCharacter(character),
            } => StableIdEditError::Reserved { id, character },
            AuthoringEditError::InvalidStableId { id, .. } => StableIdEditError::Malformed { id },
            AuthoringEditError::StableIdCollision { id, document } => match document {
                Some(catalog) => StableIdEditError::InCatalog {
                    id: id.to_string(),
                    catalog: self.display(&catalog),
                },
                None => StableIdEditError::InUse { id: id.to_string() },
            },
            AuthoringEditError::UnknownStableId { id } => StableIdEditError::Unknown { id },
            AuthoringEditError::AmbiguousStableId { id } => {
                StableIdEditError::Ambiguous { id: id.to_string() }
            }
            AuthoringEditError::MalformedCatalog { document } => {
                StableIdEditError::CatalogMalformed {
                    catalog: self.display(&document),
                }
            }
            AuthoringEditError::NoEdits => StableIdEditError::Unchanged {
                id: match target {
                    StableIdTarget::Id(id) => id.to_string(),
                    _ => String::new(),
                },
            },
            AuthoringEditError::NoSymbol { document, line, .. }
            | AuthoringEditError::AmbiguousSymbol { document, line, .. } => {
                StableIdEditError::Target {
                    target: format!("{}:{line}", self.display(&document)),
                }
            }
            _ => {
                report_diagnostics(stderr, messages, snapshot.diagnostics().iter())?;
                return Ok(CliError::Diagnostics);
            }
        };
        Ok(CliError::StableIdEdit(refusal))
    }
}

fn canonical(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CliError> {
    let mut canonical = paths
        .into_iter()
        .map(|path| std::fs::canonicalize(&path).map_err(|source| CliError::Read { path, source }))
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    Ok(canonical)
}

fn common_directory<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> PathBuf {
    paths
        .filter_map(|path| path.parent().map(Path::to_owned))
        .reduce(|left, right| {
            left.components()
                .zip(right.components())
                .take_while(|(left, right)| left == right)
                .map(|(component, _)| component.as_os_str())
                .collect()
        })
        .unwrap_or_default()
}

fn keyed(
    root: &Path,
    paths: Vec<PathBuf>,
) -> Result<BTreeMap<DocumentKey, (PathBuf, String)>, CliError> {
    let mut documents = BTreeMap::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        let relative = path
            .strip_prefix(root)
            .map_or_else(|_| display_path(&path), display_path);
        let key = DocumentKey::new(relative).map_err(|error| CliError::Read {
            path: path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
        })?;
        documents.insert(key, (path, text));
    }
    Ok(documents)
}
