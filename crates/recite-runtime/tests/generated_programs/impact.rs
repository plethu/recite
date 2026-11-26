//! Save impact against the reference save points and against real saves.

use proptest::prelude::*;
use recite_runtime::resume::{
    ResumeOptions, SavePointKind, SavePointOutcome, SessionResume, analyze_save_impact,
    resume_session,
};
use recite_testkit::{
    EffectMode, EffectObservation, Observation, ReferenceProgram, SavePointKey, answers,
    apply_patch, patch, program,
};

use super::resume::saves;
use super::support::{compile, failure};

fn key(kind: &SavePointKind) -> Result<SavePointKey, TestCaseError> {
    let ids = |ids: &[recite_core::ChoiceId]| ids.iter().map(|id| id.as_str().to_owned()).collect();
    Ok(match kind {
        SavePointKind::BlockStart => SavePointKey::BlockStart,
        SavePointKind::AfterLine { line } => SavePointKey::AfterLine(line.as_str().to_owned()),
        SavePointKind::Prompt { line, choices } => SavePointKey::Prompt {
            line: line.as_ref().map(|line| line.as_str().to_owned()),
            choices: ids(choices),
        },
        SavePointKind::BlockingEffect { function } => {
            SavePointKey::BlockingEffect(function.clone())
        }
        SavePointKind::AfterImmediateEffect { function } => {
            SavePointKey::AfterImmediateEffect(function.clone())
        }
        other => return Err(failure(other)),
    })
}

/// The save point a real save was taken at, from what it was taken after.
fn observed_key(after: Option<&Observation>) -> Option<SavePointKey> {
    Some(match after {
        None => SavePointKey::BlockStart,
        Some(Observation::Line(line)) => SavePointKey::AfterLine(line.id.clone()),
        Some(Observation::Prompt { line, choices }) => SavePointKey::Prompt {
            line: line.as_ref().map(|line| line.id.clone()),
            choices: choices.iter().map(|choice| choice.id.clone()).collect(),
        },
        Some(Observation::Effect(EffectObservation { mode, function, .. })) => match mode {
            EffectMode::Blocking => SavePointKey::BlockingEffect(function.clone()),
            _ => SavePointKey::AfterImmediateEffect(function.clone()),
        },
        Some(Observation::End { .. }) => return None,
    })
}

fn same_outcome(reported: &SavePointOutcome, resumed: &SessionResume) -> bool {
    match (reported, resumed) {
        (SavePointOutcome::Resumes { level, prompt }, SessionResume::Resumed { report, .. }) => {
            *level == report.level && *prompt == report.prompt
        }
        (
            SavePointOutcome::Fallback {
                reason,
                block_available,
            },
            SessionResume::Fallback(fallback),
        ) => *reason == fallback.reason && *block_available == fallback.block_available,
        _ => false,
    }
}

proptest! {
    /// Save impact reports exactly the reference interpreter's reachable save
    /// points, and every real save resumes as the report says.
    #[test]
    fn save_impact_matches_reachable_points_and_real_saves(
        program in program(),
        ops in patch(),
        answers in answers(),
    ) {
        let baseline = compile(&program)?;
        let candidate = compile(&apply_patch(&program, &ops))?;
        let report = analyze_save_impact(&baseline, &candidate).map_err(failure)?;

        let mut reported = report
            .points
            .iter()
            .map(|point| Ok((point.block_id.as_str().to_owned(), key(&point.kind)?)))
            .collect::<Result<Vec<_>, TestCaseError>>()?;
        reported.sort();
        let reference = ReferenceProgram::lower(&program.sources()).map_err(failure)?;
        prop_assert_eq!(reported, reference.save_points());

        for save in saves(&baseline, &answers)? {
            let Some(saved_at) = observed_key(save.after.as_ref()) else {
                continue;
            };
            let resumed = resume_session(&candidate, save.snapshot.clone(), &ResumeOptions::new())
                .map_err(failure)?;
            let block = save.snapshot.block_id.as_str();
            let reported = report.points.iter().any(|point| {
                point.block_id.as_str() == block
                    && key(&point.kind).ok().as_ref() == Some(&saved_at)
                    && same_outcome(&point.outcome, &resumed)
            });
            prop_assert!(
                reported,
                "a save at {saved_at:?} in `{block}` resumes as {resumed:?}, which save impact does not report"
            );
        }
    }
}
