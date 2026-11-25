//! `save inspect`: what a player's save holds, read without the game, and
//! optionally what it does against a compiled asset.

use std::io::Write;

use recite_runtime::resume::{PromptResume, ResumeOptions, SessionResume, resume_session};
use recite_runtime::snapshot::{
    DialogueLandmarkSnapshot, DialogueSessionSnapshot, decode_session_snapshot_messagepack,
};
use serde_json::{Value, json};

use crate::args::{ReportFormat, SaveInspectArgs};
use crate::error::{CliError, SaveError};
use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};

use super::assets;
use super::impact::{level_name, reason_text};

pub(super) fn run(
    args: &SaveInspectArgs,
    stdout: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let bytes = std::fs::read(&args.save).map_err(|source| CliError::Read {
        path: args.save.clone(),
        source,
    })?;
    let snapshot =
        decode_session_snapshot_messagepack(&bytes).map_err(|error| SaveError::Snapshot {
            path: display_path(&args.save),
            detail: error.to_string(),
        })?;
    let prediction = args
        .asset
        .as_ref()
        .map(|path| {
            let asset = assets::compiled_file(path)?;
            let outcome = resume_session(&asset, snapshot.clone(), &ResumeOptions::new())?;
            Ok::<_, CliError>((display_path(path), outcome))
        })
        .transpose()?;

    match args.format {
        ReportFormat::Json => {
            let mut document = snapshot_json(&snapshot);
            if let Some((asset, outcome)) = &prediction {
                document["prediction"] = prediction_json(asset, outcome, snapshot.ended);
            }
            serde_json::to_writer_pretty(&mut *stdout, &document)
                .map_err(|error| CliError::Io(error.into()))?;
            writeln!(stdout)?;
        }
        ReportFormat::Text => {
            write_text(stdout, messages, &snapshot)?;
            if let Some((asset, outcome)) = &prediction {
                write_prediction(stdout, messages, asset, outcome, snapshot.ended)?;
            }
        }
    }
    Ok(())
}

fn write_text(
    stdout: &mut dyn Write,
    messages: &Messages,
    snapshot: &DialogueSessionSnapshot,
) -> Result<(), CliError> {
    let position = match nearest_landmark(snapshot) {
        _ if snapshot.ended => messages.text(MsgId::CliSaveInspectPositionEnded),
        // A prompt landmark is where the save waits, not a line it follows.
        Some(landmark @ DialogueLandmarkSnapshot::Prompt { .. })
            if snapshot.pending_prompt.is_some() =>
        {
            messages.format(
                MsgId::CliSavePointPrompt,
                [("prompt", landmark_name(landmark))],
            )
        }
        Some(landmark) => messages.format(
            MsgId::CliSaveInspectPositionAfter,
            [("landmark", landmark_name(landmark))],
        ),
        None => messages.text(MsgId::CliSaveInspectPositionStart),
    };
    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::CliSaveInspectSummary,
            [
                ("asset", snapshot.asset_id.clone()),
                ("block", snapshot.block_id.clone()),
                ("position", position),
                ("format", snapshot.snapshot_format_version.to_string()),
                ("compiler", snapshot.compiler_version.clone()),
            ]
        )
    )?;
    if snapshot.ended {
        writeln!(stdout, "  {}", messages.text(MsgId::CliSaveInspectEnded))?;
    }
    if let Some(prompt) = &snapshot.pending_prompt {
        let choices = prompt
            .choices
            .iter()
            .map(|choice| format!("@{}", choice.id))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            stdout,
            "  {}",
            messages.format(MsgId::CliSaveInspectPendingPrompt, [("choices", choices)])
        )?;
    }
    if let Some(effect) = &snapshot.pending_effect {
        writeln!(
            stdout,
            "  {}",
            messages.format(
                MsgId::CliSaveInspectPendingEffect,
                [
                    ("function", effect.request.function.clone()),
                    ("id", effect.request.id.clone()),
                ]
            )
        )?;
    }
    // An ended conversation delivered its deferred effects with its end.
    if !snapshot.ended && !snapshot.deferred_effects.is_empty() {
        let functions = snapshot
            .deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            stdout,
            "  {}",
            messages.format(MsgId::CliSaveInspectDeferred, [("functions", functions)])
        )?;
    }
    Ok(())
}

fn write_prediction(
    stdout: &mut dyn Write,
    messages: &Messages,
    asset: &str,
    outcome: &SessionResume,
    ended: bool,
) -> Result<(), CliError> {
    // Adapters restore no session for an ended save, whatever the asset.
    if ended {
        writeln!(
            stdout,
            "{}",
            messages.format(
                MsgId::CliSaveInspectRestoresNothing,
                [("asset", asset.to_owned())]
            )
        )?;
        return Ok(());
    }
    match outcome {
        SessionResume::Resumed { report, .. } => {
            writeln!(
                stdout,
                "{}",
                messages.format(
                    MsgId::CliSaveInspectResumes,
                    [
                        ("asset", asset.to_owned()),
                        ("level", level_name(report.level).to_owned())
                    ]
                )
            )?;
            if report.prompt == PromptResume::Reevaluated {
                writeln!(
                    stdout,
                    "  {}",
                    messages.text(MsgId::CliSaveInspectReevaluated)
                )?;
            }
        }
        SessionResume::Fallback(fallback) => {
            writeln!(
                stdout,
                "{}",
                messages.format(
                    MsgId::CliSaveInspectFallback,
                    [
                        ("asset", asset.to_owned()),
                        ("reason", reason_text(messages, &fallback.reason)),
                    ]
                )
            )?;
        }
        _ => {}
    }
    Ok(())
}

/// The last line or prompt before the saved point: in the innermost range,
/// or else in the ranges around the branch it is inside.
fn nearest_landmark(snapshot: &DialogueSessionSnapshot) -> Option<&DialogueLandmarkSnapshot> {
    // An ended save is anchored at the end of its block, not where the
    // conversation ended, so it names no line.
    if snapshot.ended {
        return None;
    }
    let anchor = &snapshot.resume_anchor;
    std::iter::once(&anchor.position)
        .chain(anchor.frames.iter().rev().map(|frame| &frame.position))
        .find_map(|position| position.landmark.as_ref())
}

fn landmark_name(landmark: &DialogueLandmarkSnapshot) -> String {
    match landmark {
        DialogueLandmarkSnapshot::Line { id } => format!("@{id}"),
        DialogueLandmarkSnapshot::Prompt { line, choices } => line
            .as_deref()
            .or_else(|| choices.first().map(String::as_str))
            .map_or_else(String::new, |id| format!("@{id}")),
    }
}

fn snapshot_json(snapshot: &DialogueSessionSnapshot) -> Value {
    let pending = if let Some(prompt) = &snapshot.pending_prompt {
        json!({
            "kind": "prompt",
            "choices": prompt.choices.iter().map(|choice| choice.id.as_str()).collect::<Vec<_>>(),
        })
    } else if let Some(effect) = &snapshot.pending_effect {
        json!({
            "kind": "blocking_effect",
            "id": effect.request.id,
            "function": effect.request.function,
        })
    } else {
        Value::Null
    };
    json!({
        "format_version": 1,
        "asset_id": snapshot.asset_id,
        "snapshot_format_version": snapshot.snapshot_format_version,
        "compiler_version": snapshot.compiler_version,
        "block_id": snapshot.block_id,
        "position": {
            "landmark": nearest_landmark(snapshot).map(landmark_json),
            "branch_depth": snapshot.resume_anchor.frames.len(),
        },
        "pending": pending,
        // An ended conversation delivered its deferred effects with its end.
        "deferred_effects": snapshot
            .deferred_effects
            .iter()
            .filter(|_| !snapshot.ended)
            .map(|effect| json!({ "id": effect.id, "function": effect.function }))
            .collect::<Vec<_>>(),
        "selected_choice_history": snapshot.selected_choice_history,
        "locale": snapshot.locale,
        "ended": snapshot.ended,
    })
}

fn landmark_json(landmark: &DialogueLandmarkSnapshot) -> Value {
    match landmark {
        DialogueLandmarkSnapshot::Line { id } => json!({ "kind": "line", "line": id }),
        DialogueLandmarkSnapshot::Prompt { line, choices } => {
            json!({ "kind": "prompt", "line": line, "choices": choices })
        }
    }
}

fn prediction_json(asset: &str, outcome: &SessionResume, ended: bool) -> Value {
    if ended {
        return json!({ "asset": asset, "outcome": "ended" });
    }
    match outcome {
        SessionResume::Resumed { report, .. } => json!({
            "asset": asset,
            "outcome": "resumes",
            "level": level_name(report.level),
            "prompt": recite_adapter::resume_names::prompt_resume_name(report.prompt),
            "schema_changed": report.schema_changed,
            "compiler_changed": report.compiler_changed,
        }),
        SessionResume::Fallback(fallback) => json!({
            "asset": asset,
            "outcome": "fallback",
            "reason": recite_adapter::resume_names::fallback_reason_name(&fallback.reason),
            "block_available": fallback.block_available,
        }),
        _ => Value::Null,
    }
}
