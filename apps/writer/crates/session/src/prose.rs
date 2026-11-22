//! Prose findings for open documents in projects with `[prose] enabled`.
//!
//! The shared `recite-prose` crate owns settings, the engine and fix ranges, as it does for the
//! CLI and the language server. This module captures a project's configuration while its
//! discovery report is held, keeps one checker per project, caches findings per document text and
//! appends words to the project dictionary. A project that does not enable prose captures nothing,
//! so its sessions build no checker and request no work.
use recite_core::{Diagnostic, DiagnosticSeverity};
use recite_prose::{ProseChecker, ProseCheckerCache, ProseSettings, ProseSetupError};
pub use recite_prose::{ProseEdit, ProseFinding, ProseFindingKind};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Documents whose latest findings a checker keeps; reopened text is not checked again.
const MAX_CACHED_DOCUMENTS: usize = 64;

/// A project's `[prose]` table, captured only when the project enables checking.
#[derive(Debug, PartialEq)]
pub struct Configuration {
    root: PathBuf,
    settings: Result<ProseSettings, ProseSetupError>,
}

impl Configuration {
    /// `None` when the project does not enable prose checking.
    pub fn capture(report: &recite_config::ProjectDiscoveryReport) -> Option<Arc<Self>> {
        let manifest = report.manifest();
        let prose = &manifest.source().manifest().prose;
        prose.enabled.then(|| {
            let root = manifest.project_root();
            Arc::new(Self {
                root: root.to_owned(),
                settings: ProseSettings::from_project(root, prose),
            })
        })
    }

    /// The dictionary that receives added words, when the project names one.
    pub fn dictionary(&self) -> Option<&Path> {
        self.settings
            .as_ref()
            .ok()
            .and_then(ProseSettings::dictionary_for_additions)
    }

    /// Append `word` to the project dictionary, creating the file when needed. Returns `false`
    /// when the word is already listed or cannot be stored, leaving the file untouched.
    pub fn add_word(&self, word: &str) -> io::Result<bool> {
        let path = self.dictionary().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "the project's [prose] table names no dictionary",
            )
        })?;
        let existing = match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let Some(edit) = recite_prose::dictionary_addition(existing.as_deref(), word) else {
            return Ok(false);
        };
        let mut text = existing.unwrap_or_default();
        text.replace_range(edit.range, &edit.replacement);
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "dictionary has no parent")
        })?;
        std::fs::create_dir_all(parent)?;
        let mut file = atomic_write_file::AtomicWriteFile::open(path)?;
        file.write_all(text.as_bytes())?;
        file.commit()?;
        crate::sync_directory(parent)?;
        Ok(true)
    }
}

/// One document's prose result: its findings, or the reason prose cannot be checked.
#[derive(Clone, Debug)]
pub enum Outcome {
    Findings(Arc<[ProseFinding]>),
    /// A setup problem, reported once as a warning at the start of the document.
    Setup(Box<Diagnostic>),
}

impl Outcome {
    /// The problem at `index`, in finding order.
    pub fn diagnostic(&self, index: usize) -> Option<&Diagnostic> {
        match self {
            Self::Findings(findings) => findings.get(index).map(|finding| &finding.diagnostic),
            Self::Setup(diagnostic) => (index == 0).then_some(diagnostic.as_ref()),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Findings(findings) => findings.len(),
            Self::Setup(_) => 1,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The finding behind the problem at `index`; setup problems have none.
    pub fn finding(&self, index: usize) -> Option<&ProseFinding> {
        match self {
            Self::Findings(findings) => findings.get(index),
            Self::Setup(_) => None,
        }
    }
}

struct Cached {
    text: Arc<str>,
    checker: Arc<Mutex<ProseChecker>>,
    findings: Arc<[ProseFinding]>,
}

/// Checkers per project and the latest findings per document. One worker owns it.
#[derive(Default)]
pub struct Checker {
    checkers: ProseCheckerCache,
    results: BTreeMap<String, Cached>,
}

impl std::fmt::Debug for Checker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProseChecker")
            .field("documents", &self.results.len())
            .finish_non_exhaustive()
    }
}

impl Checker {
    /// Check `source`, labelled by its document `key` as the kernel labels its diagnostics.
    /// Unchanged text with an unchanged checker returns the cached findings.
    pub fn check(
        &mut self,
        configuration: &Configuration,
        key: &str,
        source: &Arc<str>,
    ) -> Outcome {
        let checker = match &configuration.settings {
            Ok(settings) => self.checkers.checker(&configuration.root, settings),
            Err(error) => Err(error.clone()),
        };
        let checker = match checker {
            Ok(checker) => checker,
            Err(error) => return setup(&error, key),
        };
        if let Some(cached) = self.results.get(key)
            && Arc::ptr_eq(&cached.checker, &checker)
            && (Arc::ptr_eq(&cached.text, source) || cached.text == *source)
        {
            return Outcome::Findings(cached.findings.clone());
        }
        let findings: Arc<[ProseFinding]> = checker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .check(key, source)
            .into();
        if self.results.len() >= MAX_CACHED_DOCUMENTS && !self.results.contains_key(key) {
            self.results.clear();
        }
        self.results.insert(
            key.to_owned(),
            Cached {
                text: source.clone(),
                checker,
                findings: findings.clone(),
            },
        );
        Outcome::Findings(findings)
    }
}

fn setup(error: &ProseSetupError, key: &str) -> Outcome {
    match error.diagnostic(key) {
        Some(mut diagnostic) => {
            diagnostic.severity = DiagnosticSeverity::Warning;
            Outcome::Setup(Box::new(diagnostic))
        }
        None => Outcome::Findings(Arc::from([])),
    }
}

#[cfg(test)]
mod tests;
