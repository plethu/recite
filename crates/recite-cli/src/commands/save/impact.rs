//! `save impact`: where players' saves resume after a dialogue patch.
//!
//! The classification is the runtime's own [`analyze_save_impact`], so this
//! command reports exactly what players' saves will do; it only renders it.

use std::collections::BTreeSet;
use std::io::Write;

use recite_runtime::resume::{
    ResumeFallbackReason, ResumeLandmark, ResumeLevel, SaveImpactReport, SavePointImpact,
    SavePointKind, SavePointOutcome, analyze_save_impact,
};
use serde_json::{Value, json};

use crate::args::{ReportFormat, SaveImpactArgs};
use crate::error::{CliError, SaveError};
use crate::i18n::{Messages, MsgId};
use recite_ui::UiArg;

use super::assets::{self, Assets};

pub(super) fn run(
    args: &SaveImpactArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let candidates = assets::load(&args.candidate, stderr, messages)?;
    let baselines = match (&args.baseline, &args.since) {
        (Some(baseline), _) => assets::load(baseline, stderr, messages)?,
        (None, Some(reference)) => {
            assets::project_at_revision(&args.candidate, reference, stderr, messages)?
        }
        (None, None) => unreachable!("clap requires --baseline or --since"),
    };
    let reports = compare(&baselines, &candidates)?;
    let allowed = args
        .allow_fallback
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let unacknowledged = reports
        .iter()
        .flat_map(|(_, report)| report.fallbacks())
        .filter(|point| !allowed.contains(point.block_id.as_str()))
        .count();

    match args.format {
        ReportFormat::Json => {
            let document = json!({
                "format_version": 1,
                "assets": reports
                    .iter()
                    .map(|(asset, report)| asset_json(asset, report, &allowed))
                    .collect::<Vec<_>>(),
                "unacknowledged_fallbacks": unacknowledged,
            });
            serde_json::to_writer_pretty(&mut *stdout, &document)
                .map_err(|error| CliError::Io(error.into()))?;
            writeln!(stdout)?;
        }
        ReportFormat::Text => {
            for (asset, report) in &reports {
                write_text(stdout, messages, asset, report, &allowed)?;
            }
        }
    }

    if args.check && unacknowledged > 0 {
        writeln!(
            stderr,
            "{}",
            messages.format(
                MsgId::CliSaveImpactUnacknowledged,
                [("count", UiArg::from(unacknowledged))]
            )
        )?;
        return Err(CliError::Diagnostics);
    }
    Ok(())
}

pub(super) fn write_reports(
    out: &mut dyn Write,
    messages: &Messages,
    baselines: &Assets,
    candidates: &Assets,
) -> Result<(), CliError> {
    for (asset, report) in compare(baselines, candidates)? {
        write_text(out, messages, &asset, &report, &BTreeSet::new())?;
    }
    Ok(())
}

/// Pairs each baseline asset with the candidate asset of the same asset ID.
fn compare(
    baselines: &Assets,
    candidates: &Assets,
) -> Result<Vec<(String, SaveImpactReport)>, CliError> {
    baselines
        .iter()
        .map(|(asset, baseline)| {
            let candidate = candidates
                .iter()
                .find(|(candidate, _)| candidate == asset)
                .map(|(_, dialogue)| dialogue)
                .ok_or_else(|| SaveError::MissingAsset {
                    asset: asset.clone(),
                })?;
            let report = analyze_save_impact(baseline, candidate)?;
            Ok((asset.clone(), report))
        })
        .collect()
}

fn write_text(
    stdout: &mut dyn Write,
    messages: &Messages,
    asset: &str,
    report: &SaveImpactReport,
    allowed: &BTreeSet<&str>,
) -> Result<(), CliError> {
    let summary = report.summary();
    if report.identical {
        writeln!(
            stdout,
            "{}",
            messages.format(
                MsgId::CliSaveImpactIdentical,
                [
                    ("asset", UiArg::from(asset.to_owned())),
                    ("total", UiArg::from(summary.total))
                ]
            )
        )?;
        return Ok(());
    }
    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::CliSaveImpactSummary,
            [
                ("asset", UiArg::from(asset.to_owned())),
                ("total", UiArg::from(summary.total)),
                ("exact", UiArg::from(summary.exact)),
                ("unchanged", UiArg::from(summary.unchanged_block)),
                ("text", UiArg::from(summary.text_edited)),
                ("relocated", UiArg::from(summary.relocated)),
                ("fallback", UiArg::from(summary.fallback)),
            ]
        )
    )?;
    if summary.reevaluated_prompts > 0 {
        writeln!(
            stdout,
            "  {}",
            messages.format(
                MsgId::CliSaveImpactPrompts,
                [("count", UiArg::from(summary.reevaluated_prompts))]
            )
        )?;
    }
    for point in report.fallbacks() {
        let SavePointOutcome::Fallback { reason, .. } = &point.outcome else {
            continue;
        };
        let span = &point.baseline_span;
        writeln!(
            stdout,
            "{}",
            messages.format(
                MsgId::CliSaveImpactFallback,
                [
                    (
                        "location",
                        format!(
                            "{}:{}:{}",
                            span.file,
                            span.start.line(),
                            span.start.column()
                        )
                    ),
                    ("block", point.block_id.as_str().to_owned()),
                    ("point", point_text(messages, &point.kind)),
                    ("reason", reason_text(messages, reason)),
                ]
            )
        )?;
        if allowed.contains(point.block_id.as_str()) {
            writeln!(
                stdout,
                "  {}",
                messages.format(
                    MsgId::CliSaveImpactAcknowledged,
                    [("block", point.block_id.as_str().to_owned())]
                )
            )?;
        }
    }
    Ok(())
}

pub(super) fn point_text(messages: &Messages, kind: &SavePointKind) -> String {
    match kind {
        SavePointKind::BlockStart => messages.text(MsgId::CliSavePointBlockStart),
        SavePointKind::AfterLine { line } => messages.format(
            MsgId::CliSavePointAfterLine,
            [("line", line.as_str().to_owned())],
        ),
        SavePointKind::Prompt { line, choices } => messages.format(
            MsgId::CliSavePointPrompt,
            [(
                "prompt",
                prompt_name(
                    line.as_ref().map(|line| line.as_str()),
                    choices.iter().map(|choice| choice.as_str()),
                ),
            )],
        ),
        SavePointKind::BlockingEffect { function } => messages.format(
            MsgId::CliSavePointBlockingEffect,
            [("function", function.clone())],
        ),
        SavePointKind::AfterImmediateEffect { function } => messages.format(
            MsgId::CliSavePointAfterEffect,
            [("function", function.clone())],
        ),
        _ => String::new(),
    }
}

pub(super) fn reason_text(messages: &Messages, reason: &ResumeFallbackReason) -> String {
    match reason {
        ResumeFallbackReason::BlockRemoved => messages.text(MsgId::CliSaveReasonBlockRemoved),
        ResumeFallbackReason::LandmarkRemoved { landmark } => messages.format(
            MsgId::CliSaveReasonLandmarkRemoved,
            [("landmark", landmark_name(landmark))],
        ),
        ResumeFallbackReason::LandmarkMoved { landmark } => messages.format(
            MsgId::CliSaveReasonLandmarkMoved,
            [("landmark", landmark_name(landmark))],
        ),
        ResumeFallbackReason::LandmarkAmbiguous { landmark } => messages.format(
            MsgId::CliSaveReasonLandmarkAmbiguous,
            [("landmark", landmark_name(landmark))],
        ),
        ResumeFallbackReason::StatementsChanged {
            landmark: Some(landmark),
        } => messages.format(
            MsgId::CliSaveReasonStatementsChanged,
            [("landmark", landmark_name(landmark))],
        ),
        ResumeFallbackReason::StatementsChanged { landmark: None } => {
            messages.text(MsgId::CliSaveReasonStatementsChangedStart)
        }
        ResumeFallbackReason::BranchRemoved => messages.text(MsgId::CliSaveReasonBranchRemoved),
        ResumeFallbackReason::LevelRefused { required, allowed } => messages.format(
            MsgId::CliSaveReasonLevelRefused,
            [
                ("required", level_name(*required).to_owned()),
                ("allowed", level_name(*allowed).to_owned()),
            ],
        ),
        _ => String::new(),
    }
}

fn landmark_name(landmark: &ResumeLandmark) -> String {
    match landmark {
        ResumeLandmark::Line(line) => format!("@{line}"),
        ResumeLandmark::Prompt { line, choices } => prompt_name(
            line.as_ref().map(|line| line.as_str()),
            choices.iter().map(|choice| choice.as_str()),
        ),
        _ => String::new(),
    }
}

/// A prompt is named by its line, or by its first choice when it has none.
fn prompt_name<'a>(line: Option<&str>, mut choices: impl Iterator<Item = &'a str>) -> String {
    line.or_else(|| choices.next())
        .map_or_else(String::new, |id| format!("@{id}"))
}

pub(super) fn level_name(level: ResumeLevel) -> &'static str {
    recite_adapter::resume_names::resume_level_name(level)
}

fn asset_json(asset: &str, report: &SaveImpactReport, allowed: &BTreeSet<&str>) -> Value {
    let summary = report.summary();
    json!({
        "asset_id": asset,
        "identical": report.identical,
        "schema_changed": report.schema_changed,
        "compiler_changed": report.compiler_changed,
        "summary": {
            "total": summary.total,
            "exact": summary.exact,
            "unchanged_block": summary.unchanged_block,
            "text_edited": summary.text_edited,
            "relocated": summary.relocated,
            "reevaluated_prompts": summary.reevaluated_prompts,
            "fallback": summary.fallback,
        },
        "save_points": report
            .points
            .iter()
            .map(|point| point_json(point, allowed))
            .collect::<Vec<_>>(),
    })
}

fn point_json(point: &SavePointImpact, allowed: &BTreeSet<&str>) -> Value {
    let span = &point.baseline_span;
    let mut object = serde_json::Map::new();
    object.insert("block_id".into(), point.block_id.as_str().into());
    object.insert(
        "source".into(),
        json!({
            "file": span.file,
            "line": span.start.line(),
            "column": span.start.column(),
        }),
    );
    match &point.kind {
        SavePointKind::BlockStart => {
            object.insert("kind".into(), "block_start".into());
        }
        SavePointKind::AfterLine { line } => {
            object.insert("kind".into(), "after_line".into());
            object.insert("line".into(), line.as_str().into());
        }
        SavePointKind::Prompt { line, choices } => {
            object.insert("kind".into(), "prompt".into());
            object.insert(
                "line".into(),
                line.as_ref().map(|line| line.as_str()).into(),
            );
            object.insert(
                "choices".into(),
                choices
                    .iter()
                    .map(|choice| choice.as_str())
                    .collect::<Vec<_>>()
                    .into(),
            );
        }
        SavePointKind::BlockingEffect { function } => {
            object.insert("kind".into(), "blocking_effect".into());
            object.insert("function".into(), function.as_str().into());
        }
        SavePointKind::AfterImmediateEffect { function } => {
            object.insert("kind".into(), "after_immediate_effect".into());
            object.insert("function".into(), function.as_str().into());
        }
        _ => {}
    }
    match &point.outcome {
        SavePointOutcome::Resumes { level, prompt } => {
            object.insert("outcome".into(), "resumes".into());
            object.insert("level".into(), level_name(*level).into());
            object.insert(
                "prompt".into(),
                recite_adapter::resume_names::prompt_resume_name(*prompt).into(),
            );
        }
        SavePointOutcome::Fallback {
            reason,
            block_available,
        } => {
            object.insert("outcome".into(), "fallback".into());
            object.insert(
                "reason".into(),
                recite_adapter::resume_names::fallback_reason_name(reason).into(),
            );
            object.insert("block_available".into(), (*block_available).into());
            object.insert(
                "acknowledged".into(),
                allowed.contains(point.block_id.as_str()).into(),
            );
        }
        _ => {}
    }
    Value::Object(object)
}
