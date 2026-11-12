//! `recording import`: validate returned rows, then optionally fold them into the sidecar.

use std::io::Write;
use std::path::Path;

use recite_compiler::authoring::{ImportRefusal, plan_import};
use recite_core::recording::HandoffRow;
use recite_ui::UiArg;

use super::Recording;
use crate::args::{HandoffFormat, RecordingImportArgs};
use crate::error::{CliError, LocalisationError};
use crate::fs::{display_path, write_staged};
use crate::i18n::{Messages, MsgId};

pub(super) fn run(
    args: &RecordingImportArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let recording = Recording::load(
        &args.project_root,
        args.language.locale.as_deref(),
        stderr,
        messages,
    )?;
    let text = std::fs::read_to_string(&args.file).map_err(|source| CliError::Read {
        path: args.file.clone(),
        source,
    })?;
    let format = args.format.unwrap_or_else(|| {
        if args
            .file
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            HandoffFormat::Json
        } else {
            HandoffFormat::Csv
        }
    });
    let rows = match format {
        HandoffFormat::Csv => HandoffRow::from_csv(&text),
        HandoffFormat::Json => HandoffRow::from_json(&text),
    }
    .map_err(|error| {
        CliError::Localisation(LocalisationError::RecordingHandoff {
            file: display_path(&args.file),
            detail: error.to_string(),
        })
    })?;

    let root = recording.root.clone();
    let media_exists = |reference: &str| media_in_project(&root, reference);
    let plan = match plan_import(
        &recording.items,
        &recording.sidecar,
        recording.locale.as_deref(),
        &rows,
        &media_exists,
    ) {
        Ok(plan) => plan,
        Err(refusals) => {
            for refusal in refusals {
                let reason = match &refusal.reason {
                    ImportRefusal::UnknownId => messages.text(MsgId::CliRecordingReasonUnknownId),
                    ImportRefusal::WrongLocale { found } => messages.format(
                        MsgId::CliRecordingReasonWrongLocale,
                        [("found", found.clone())],
                    ),
                    ImportRefusal::DuplicateTake => messages.format(
                        MsgId::CliRecordingReasonDuplicateTake,
                        [("take", refusal.take.clone())],
                    ),
                    ImportRefusal::NotReady => messages.text(MsgId::CliRecordingReasonNotReady),
                    ImportRefusal::StaleRevision { current } => messages.format(
                        MsgId::CliRecordingReasonStale,
                        [("current", current.clone())],
                    ),
                    ImportRefusal::MissingMedia => messages.format(
                        MsgId::CliRecordingReasonMissingMedia,
                        [("take", refusal.take.clone())],
                    ),
                };
                writeln!(
                    stderr,
                    "{}",
                    messages.format(
                        MsgId::CliRecordingImportRefused,
                        [
                            ("row", UiArg::from(refusal.row)),
                            ("id", UiArg::from(refusal.id.clone())),
                            ("reason", UiArg::from(reason)),
                        ],
                    )
                )?;
            }
            return Err(CliError::Diagnostics);
        }
    };
    writeln!(
        stderr,
        "{}",
        messages.format(
            MsgId::CliRecordingImportSummary,
            [
                ("added", UiArg::from(plan.added.len())),
                ("present", UiArg::from(plan.already_present)),
            ],
        )
    )?;
    let sidecar = [("sidecar", recording.sidecar_display.clone())];
    if !args.apply {
        writeln!(
            stderr,
            "{}",
            messages.format(MsgId::CliRecordingImportPreview, sidecar)
        )?;
        return Ok(());
    }
    if !plan.added.is_empty() {
        unchanged_since_read(&recording)?;
        if let Some(parent) = recording.sidecar_path.parent() {
            std::fs::create_dir_all(parent).map_err(CliError::Io)?;
        }
        write_staged(
            &recording.sidecar_path,
            plan.sidecar.to_toml_string().as_bytes(),
        )?;
    }
    writeln!(
        stderr,
        "{}",
        messages.format(MsgId::CliRecordingImportApplied, sidecar)
    )?;
    Ok(())
}

/// A take reference names an existing file inside the project root.
fn media_in_project(root: &Path, reference: &str) -> bool {
    std::fs::canonicalize(root.join(reference))
        .is_ok_and(|path| path.starts_with(root) && path.is_file())
}

/// Refuse to replace a sidecar that someone changed after it was read.
fn unchanged_since_read(recording: &Recording) -> Result<(), CliError> {
    let current = match std::fs::read_to_string(&recording.sidecar_path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(CliError::Read {
                path: recording.sidecar_path.clone(),
                source,
            });
        }
    };
    if current == recording.sidecar_source {
        Ok(())
    } else {
        Err(CliError::Localisation(
            LocalisationError::RecordingSidecar {
                sidecar: recording.sidecar_display.clone(),
                detail: "it changed during the import; run the import again".to_owned(),
            },
        ))
    }
}
