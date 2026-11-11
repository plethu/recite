//! `catalog status` and `catalog refresh`: dialogue PO catalogues against current source.
//!
//! Both commands extract the project's current template exactly as Writer does and refresh each
//! catalogue in memory with the shared `PoDocument::refreshed` kernel. Status reports what the
//! refreshed catalogue needs from a person; refresh writes it.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use recite_compiler::pot::{extract_pot, extract_pot_with_schema};
use recite_core::po::{PoDocument, PoEntry, PoReviewState, PotDocument};
use serde_json::json;

use crate::args::{CatalogCommand, CatalogProjectArgs, CatalogStatusArgs, ReportFormat};
use crate::diagnostics::report_diagnostics;
use crate::error::{CliError, LocalisationError};
use crate::fs::write_staged;

use super::project_files::{CatalogFile as Catalog, ProjectFiles};
use crate::i18n::{Messages, MsgId};

mod report;

pub(super) fn run(
    command: CatalogCommand,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        CatalogCommand::Status(args) => status(&args, stdout, stderr, messages),
        CatalogCommand::Refresh(args) => refresh(&args, stderr, messages),
    }
}

/// What a person still has to do for one entry after a refresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Attention {
    /// The current source has the entry but the catalogue does not yet.
    Missing,
    Review(PoReviewState),
}

impl Attention {
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Review(PoReviewState::Untranslated) => "untranslated",
            Self::Review(PoReviewState::SourceChanged) => "source_changed",
            Self::Review(PoReviewState::Reviewed) => "reviewed",
            Self::Review(_) => "needs_review",
        }
    }

    pub(super) fn label(self, messages: &Messages) -> String {
        messages.text(match self {
            Self::Missing => MsgId::CliCatalogStateMissing,
            Self::Review(PoReviewState::Untranslated) => MsgId::CliCatalogStateUntranslated,
            Self::Review(PoReviewState::SourceChanged) => MsgId::CliCatalogStateSourceChanged,
            Self::Review(_) => MsgId::CliCatalogStateNeedsReview,
        })
    }
}

struct Project {
    pot: PotDocument,
    template: PoDocument,
    catalogs: Vec<Catalog>,
}

impl Project {
    fn load(
        args: &CatalogProjectArgs,
        stderr: &mut dyn Write,
        messages: &Messages,
    ) -> Result<Self, CliError> {
        let files = ProjectFiles::load(&args.project_root, &args.catalogs, stderr, messages)?;
        let extracted = match &files.schema {
            Some(schema) => extract_pot_with_schema(files.inputs(), schema),
            None => extract_pot(files.inputs()),
        };
        report_diagnostics(stderr, messages, extracted.diagnostics.iter())?;
        let pot = extracted.catalog.ok_or(CliError::Diagnostics)?;
        let template = PoDocument::parse(pot.to_pot_string()).map_err(|error| {
            CliError::Localisation(LocalisationError::CatalogMalformed {
                catalog: "POT".to_owned(),
                detail: error.to_string(),
            })
        })?;
        Ok(Self {
            pot,
            template,
            catalogs: files.catalogs,
        })
    }

    fn refreshed(&self, catalog: &Catalog) -> Result<PoDocument, CliError> {
        catalog.document.refreshed(&self.template).map_err(|error| {
            CliError::Localisation(LocalisationError::CatalogRefresh {
                catalog: catalog.display.clone(),
                detail: error.to_string(),
            })
        })
    }
}

fn active(document: &PoDocument) -> impl Iterator<Item = (&str, &PoEntry)> {
    document
        .entries()
        .iter()
        .filter(|entry| !entry.is_header() && !entry.is_obsolete())
        .filter_map(|entry| entry.context().map(|context| (context, entry)))
}

pub(super) struct Item {
    pub(super) context: String,
    pub(super) attention: Attention,
    pub(super) reference: Option<String>,
}

pub(super) struct Status {
    pub(super) locale: Option<String>,
    pub(super) refresh_needed: bool,
    pub(super) reviewed: usize,
    pub(super) removed: usize,
    pub(super) items: Vec<Item>,
}

fn catalog_status(project: &Project, catalog: &Catalog) -> Result<Status, CliError> {
    let refreshed = project.refreshed(catalog)?;
    let before = active(&catalog.document)
        .map(|(context, _)| context)
        .collect::<BTreeSet<_>>();
    let after = active(&refreshed)
        .map(|(context, _)| context)
        .collect::<BTreeSet<_>>();
    let references = project
        .pot
        .entries
        .iter()
        .filter_map(|entry| {
            let reference = entry.reference.as_ref()?;
            Some((
                entry.context.as_str(),
                format!("{}:{}:{}", reference.file, reference.line, reference.column),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut reviewed = 0;
    let mut items = Vec::new();
    for (context, entry) in active(&refreshed) {
        let attention = if before.contains(context) {
            Attention::Review(entry.review_state())
        } else {
            Attention::Missing
        };
        if attention == Attention::Review(PoReviewState::Reviewed) {
            reviewed += 1;
            continue;
        }
        // Variant contexts follow their base entry's source location.
        let base = context.split_once('&').map_or(context, |(base, _)| base);
        items.push(Item {
            context: context.to_owned(),
            attention,
            reference: references.get(base).cloned(),
        });
    }
    Ok(Status {
        locale: catalog.locale(),
        refresh_needed: refreshed.source() != catalog.source,
        reviewed,
        removed: before.difference(&after).count(),
        items,
    })
}

fn status(
    args: &CatalogStatusArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let project = Project::load(&args.project, stderr, messages)?;
    if project.catalogs.is_empty() {
        writeln!(stderr, "{}", messages.text(MsgId::CliIdsNoCatalogs))?;
    }
    let mut clean = true;
    let mut catalogs = Vec::new();
    for catalog in &project.catalogs {
        let status = catalog_status(&project, catalog)?;
        clean &= status.items.is_empty() && !status.refresh_needed;
        match args.format {
            ReportFormat::Text => report::write_text(stdout, messages, catalog, &status)?,
            ReportFormat::Json => catalogs.push(report::json_status(catalog, &status)),
        }
    }
    if args.format == ReportFormat::Json {
        let document = json!({ "catalogs": catalogs });
        serde_json::to_writer_pretty(&mut *stdout, &document)
            .map_err(|error| CliError::Io(error.into()))?;
        writeln!(stdout)?;
    }
    if args.check && !clean {
        return Err(CliError::Diagnostics);
    }
    Ok(())
}

fn refresh(
    args: &CatalogProjectArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let project = Project::load(args, stderr, messages)?;
    if project.catalogs.is_empty() {
        writeln!(stderr, "{}", messages.text(MsgId::CliIdsNoCatalogs))?;
    }
    // Every catalogue refreshes in memory before any is written, so one refusal writes nothing.
    let refreshed = project
        .catalogs
        .iter()
        .map(|catalog| Ok((catalog, project.refreshed(catalog)?)))
        .collect::<Result<Vec<_>, CliError>>()?;
    for (catalog, document) in refreshed {
        if document.source() == catalog.source {
            writeln!(
                stderr,
                "{}",
                messages.format(
                    MsgId::CliCatalogCurrent,
                    [("catalog", catalog.display.clone())]
                )
            )?;
            continue;
        }
        unchanged_since_read(catalog)?;
        write_staged(&catalog.path, document.source().as_bytes())?;
        writeln!(
            stderr,
            "{}",
            messages.format(
                MsgId::CliCatalogRefreshed,
                [("catalog", catalog.display.clone())]
            )
        )?;
    }
    Ok(())
}

/// Refuse to replace a catalogue that someone saved after it was read.
fn unchanged_since_read(catalog: &Catalog) -> Result<(), CliError> {
    let current = std::fs::read_to_string(&catalog.path).map_err(|source| CliError::Read {
        path: catalog.path.clone(),
        source,
    })?;
    if current == catalog.source {
        Ok(())
    } else {
        Err(CliError::Localisation(LocalisationError::CatalogChanged {
            catalog: catalog.display.clone(),
        }))
    }
}
