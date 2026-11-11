//! `recording status`, `recording export` and `recording import` (spec §9.9, §13.11).
//!
//! The shared authoring layer owns items, states and import validation; this module performs the
//! explicit file I/O around them and reports results.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::authoring::{
    RecordingItem, RecordingState, handoff_rows, recording_items, recording_report,
};
use recite_compiler::compile::{compile_inputs, compile_inputs_with_schema};
use recite_core::recording::{HandoffRow, RecordingSidecar};
use serde_json::json;

use crate::args::{
    ExportState, HandoffFormat, RecordingCommand, RecordingExportArgs, RecordingStatusArgs,
    ReportFormat,
};
use crate::diagnostics::report_diagnostics;
use crate::error::{CliError, LocalisationError};
use crate::fs::{compile_options, display_path, write_staged};
use crate::i18n::{Messages, MsgId};
use recite_ui::UiArg;

use super::project_files::ProjectFiles;

mod import;

pub(super) fn run(
    command: RecordingCommand,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        RecordingCommand::Status(args) => status(&args, stdout, stderr, messages),
        RecordingCommand::Export(args) => export(&args, stderr, messages),
        RecordingCommand::Import(args) => import::run(&args, stderr, messages),
    }
}

/// One recording language's current items and stored sidecar.
struct Recording {
    root: PathBuf,
    locale: Option<String>,
    items: Vec<RecordingItem>,
    sidecar: RecordingSidecar,
    /// The sidecar text as read, or `None` when it does not exist yet.
    sidecar_source: Option<String>,
    sidecar_path: PathBuf,
    sidecar_display: String,
}

impl Recording {
    fn load(
        project_root: &Path,
        locale: Option<&str>,
        stderr: &mut dyn Write,
        messages: &Messages,
    ) -> Result<Self, CliError> {
        let files = ProjectFiles::load(project_root, &[], stderr, messages)?;
        let options = compile_options(Path::new("recording.recitec"), files.schema.as_ref())?;
        let report = match &files.schema {
            Some(schema) => compile_inputs_with_schema(files.inputs(), options, schema),
            None => compile_inputs(files.inputs(), options),
        }?;
        report_diagnostics(stderr, messages, report.diagnostics.iter())?;
        let dialogue = report.asset.ok_or(CliError::Diagnostics)?.dialogue;
        let catalog = match locale {
            None => None,
            Some(locale) => Some(
                &files
                    .catalogs
                    .iter()
                    .find(|catalog| catalog.locale().as_deref() == Some(locale))
                    .ok_or_else(|| {
                        CliError::Localisation(LocalisationError::RecordingLocale {
                            locale: locale.to_owned(),
                        })
                    })?
                    .document,
            ),
        };
        let items = recording_items(&dialogue, &files.direction, catalog);

        let relative = format!("recording/{}.toml", locale.unwrap_or("source"));
        let sidecar_path = files.root.join(&relative);
        let sidecar_error = |detail: String| {
            CliError::Localisation(LocalisationError::RecordingSidecar {
                sidecar: relative.clone(),
                detail,
            })
        };
        let sidecar_source = match std::fs::read_to_string(&sidecar_path) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(CliError::Read {
                    path: sidecar_path,
                    source,
                });
            }
        };
        let sidecar = match &sidecar_source {
            Some(source) => {
                let sidecar = RecordingSidecar::parse(source)
                    .map_err(|error| sidecar_error(error.to_string()))?;
                if sidecar.locale.as_deref() != locale {
                    return Err(sidecar_error(format!(
                        "it declares locale `{}`",
                        sidecar.locale.as_deref().unwrap_or("")
                    )));
                }
                sidecar
            }
            None => RecordingSidecar::new(locale.map(str::to_owned)),
        };
        Ok(Self {
            root: files.root,
            locale: locale.map(str::to_owned),
            items,
            sidecar,
            sidecar_source,
            sidecar_path,
            sidecar_display: relative,
        })
    }

    fn language(&self, messages: &Messages) -> String {
        self.locale
            .clone()
            .unwrap_or_else(|| messages.text(MsgId::CliRecordingSourceLanguage))
    }
}

fn reference(item: &RecordingItem) -> String {
    format!("{}:{}", item.file, item.line)
}

fn status(
    args: &RecordingStatusArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let recording = Recording::load(
        &args.project_root,
        args.language.locale.as_deref(),
        stderr,
        messages,
    )?;
    let entries = recording_report(&recording.items, &recording.sidecar);
    let count = |state: RecordingState| entries.iter().filter(|entry| entry.state == state).count();
    let attention = entries
        .iter()
        .filter(|entry| entry.state.needs_attention())
        .collect::<Vec<_>>();
    match args.format {
        ReportFormat::Text => {
            writeln!(
                stdout,
                "{}",
                messages.format(
                    MsgId::CliRecordingSummary,
                    [
                        ("language", UiArg::from(recording.language(messages))),
                        ("recorded", UiArg::from(count(RecordingState::Recorded))),
                        ("approved", UiArg::from(count(RecordingState::Approved))),
                        ("unvoiced", UiArg::from(count(RecordingState::Unvoiced))),
                        ("not_ready", UiArg::from(count(RecordingState::NotReady))),
                        ("attention", UiArg::from(attention.len())),
                    ],
                )
            )?;
            for entry in &attention {
                let line = match entry.item {
                    Some(item) => messages.format(
                        MsgId::CliRecordingEntry,
                        [
                            ("state", state_label(entry.state, messages)),
                            ("id", entry.id.clone()),
                            ("block", item.block.clone()),
                            ("reference", reference(item)),
                        ],
                    ),
                    None => {
                        messages.format(MsgId::CliRecordingObsolete, [("id", entry.id.clone())])
                    }
                };
                writeln!(stdout, "{line}")?;
            }
        }
        ReportFormat::Json => {
            let mut counts = BTreeMap::new();
            let mut blocks = BTreeMap::<&str, BTreeMap<&str, usize>>::new();
            let mut speakers = BTreeMap::<&str, BTreeMap<&str, usize>>::new();
            for entry in &entries {
                *counts.entry(entry.state.key()).or_insert(0) += 1;
                if let Some(item) = entry.item {
                    *blocks
                        .entry(item.block.as_str())
                        .or_default()
                        .entry(entry.state.key())
                        .or_insert(0) += 1;
                    *speakers
                        .entry(item.speaker.as_deref().unwrap_or(""))
                        .or_default()
                        .entry(entry.state.key())
                        .or_insert(0) += 1;
                }
            }
            let document = json!({
                "locale": recording.locale,
                "sidecar": recording.sidecar_display,
                "counts": counts,
                "blocks": blocks,
                "speakers": speakers,
                "attention": attention.iter().map(|entry| json!({
                    "id": entry.id,
                    "state": entry.state.key(),
                    "block": entry.item.map(|item| item.block.clone()),
                    "reference": entry.item.map(reference),
                })).collect::<Vec<_>>(),
            });
            serde_json::to_writer_pretty(&mut *stdout, &document)
                .map_err(|error| CliError::Io(error.into()))?;
            writeln!(stdout)?;
        }
    }
    if args.check && !attention.is_empty() {
        return Err(CliError::Diagnostics);
    }
    Ok(())
}

fn state_label(state: RecordingState, messages: &Messages) -> String {
    match state {
        RecordingState::ToRecord => messages.text(MsgId::CliRecordingStateToRecord),
        RecordingState::Stale => messages.text(MsgId::CliRecordingStateStale),
        other => other.key().to_owned(),
    }
}

fn export(
    args: &RecordingExportArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let recording = Recording::load(
        &args.project_root,
        args.language.locale.as_deref(),
        stderr,
        messages,
    )?;
    let states = if args.states.is_empty() {
        vec![ExportState::ToRecord, ExportState::Stale]
    } else {
        args.states.clone()
    };
    let states = states
        .into_iter()
        .map(|state| match state {
            ExportState::ToRecord => RecordingState::ToRecord,
            ExportState::Stale => RecordingState::Stale,
            ExportState::Recorded => RecordingState::Recorded,
            ExportState::Approved => RecordingState::Approved,
        })
        .collect::<BTreeSet<_>>();
    let blocks = args
        .blocks
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let selected = recording_report(&recording.items, &recording.sidecar)
        .into_iter()
        .filter(|entry| states.contains(&entry.state))
        .filter(|entry| {
            blocks.is_empty()
                || entry
                    .item
                    .is_some_and(|item| blocks.contains(item.block.as_str()))
        })
        .collect::<Vec<_>>();
    let rows = handoff_rows(&selected, recording.locale.as_deref(), &args.handoff);

    writeln!(
        stderr,
        "{}",
        messages.format(
            MsgId::CliRecordingExportSelected,
            [
                ("count", UiArg::from(rows.len())),
                ("language", UiArg::from(recording.language(messages))),
            ],
        )
    )?;
    for row in &rows {
        writeln!(
            stderr,
            "{}",
            messages.format(
                MsgId::CliRecordingExportItem,
                [("id", row.id.clone()), ("state", row.state.clone())],
            )
        )?;
    }
    let Some(output) = &args.output else {
        writeln!(
            stderr,
            "{}",
            messages.text(MsgId::CliRecordingExportPreview)
        )?;
        return Ok(());
    };
    let encoded = match args.format {
        HandoffFormat::Csv => HandoffRow::to_csv(&rows),
        HandoffFormat::Json => HandoffRow::to_json(&rows),
    };
    write_staged(output, encoded.as_bytes())?;
    writeln!(
        stderr,
        "{}",
        messages.format(
            MsgId::CliRecordingExportWritten,
            [("output", display_path(output))]
        )
    )?;
    Ok(())
}
