//! The discovered project files that catalogue and recording commands read.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::compile::CompileInput;
use recite_config::discover_project;
use recite_core::po::PoDocument;
use recite_core::schema::ProjectSchema;

use crate::diagnostics::report_diagnostics;
use crate::error::{CliError, LocalisationError};
use crate::fs::display_path;
use crate::i18n::Messages;

/// One dialogue PO catalogue as read from disk.
pub(super) struct CatalogFile {
    /// Project-relative path where possible, used in every message and report.
    pub(super) display: String,
    pub(super) path: PathBuf,
    pub(super) source: String,
    pub(super) document: PoDocument,
}

impl CatalogFile {
    /// The catalogue's `Language` header, if it names one.
    pub(super) fn locale(&self) -> Option<String> {
        self.document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Language"))
            .map(|header| header.value().trim().to_owned())
            .filter(|value| !value.is_empty())
    }
}

/// A completely discovered project: sources, schema, catalogues and recording settings.
pub(super) struct ProjectFiles {
    pub(super) root: PathBuf,
    pub(super) schema: Option<ProjectSchema>,
    documents: Vec<(String, String)>,
    pub(super) direction: Vec<String>,
    pub(super) catalogs: Vec<CatalogFile>,
}

impl ProjectFiles {
    /// Discover the project, refusing partial coverage, an invalid schema or a malformed
    /// catalogue. `extra_catalogs` adds catalogues beyond `[discovery].catalogs`.
    pub(super) fn load(
        project_root: &Path,
        extra_catalogs: &[PathBuf],
        stderr: &mut dyn Write,
        messages: &Messages,
    ) -> Result<Self, CliError> {
        let report = discover_project(project_root)
            .map_err(|source| CliError::ProjectDiscovery { source })?;
        // A partial source walk would report present entries as removed.
        if !report.is_complete() {
            let diagnostics = report
                .diagnostics()
                .iter()
                .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                .collect::<Vec<_>>();
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Err(CliError::Diagnostics);
        }
        let manifest = report.manifest();
        let root = manifest.project_root().to_owned();
        let schema = match manifest
            .load_schema()
            .map_err(|source| CliError::ProjectSchema { source })?
        {
            Some(loaded) => {
                let loaded = loaded.into_report();
                if !loaded.diagnostics.is_empty() {
                    report_diagnostics(stderr, messages, loaded.diagnostics.iter())?;
                    return Err(CliError::Diagnostics);
                }
                loaded.schema
            }
            None => None,
        };
        let documents = report
            .documents()
            .iter()
            .map(|document| {
                (
                    document.key().as_str().to_owned(),
                    document.text().to_owned(),
                )
            })
            .collect();
        let direction = manifest.source().manifest().recording.direction.clone();

        let mut paths = report
            .catalogs()
            .iter()
            .map(|catalog| catalog.path().to_owned())
            .collect::<Vec<_>>();
        paths.extend(extra_catalogs.iter().cloned());
        let mut seen = BTreeSet::new();
        let mut catalogs = Vec::new();
        for path in paths {
            let path = std::fs::canonicalize(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            if !seen.insert(path.clone()) {
                continue;
            }
            let source = std::fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            let display = display_path(path.strip_prefix(&root).unwrap_or(&path));
            let document = PoDocument::parse(source.clone()).map_err(|error| {
                CliError::Localisation(LocalisationError::CatalogMalformed {
                    catalog: display.clone(),
                    detail: error.to_string(),
                })
            })?;
            catalogs.push(CatalogFile {
                display,
                path,
                source,
                document,
            });
        }
        catalogs.sort_by(|left, right| left.display.cmp(&right.display));
        Ok(Self {
            root,
            schema,
            documents,
            direction,
            catalogs,
        })
    }

    /// Compiler inputs keyed by project-relative document key, as Writer keys them.
    pub(super) fn inputs(&self) -> impl Iterator<Item = CompileInput> + '_ {
        self.documents
            .iter()
            .map(|(key, text)| CompileInput::new(key.as_str(), text.as_str()))
    }
}
