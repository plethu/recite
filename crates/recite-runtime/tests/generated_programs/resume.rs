//! Saves taken along a run of a generated program, resumed in the same asset
//! and in a patched revision.

use proptest::prelude::*;
use recite_core::compiled::CompiledDialogue;
use recite_runtime::{
    EffectAck, acknowledge_effect,
    resume::{PromptResume, ResumeLevel, ResumeOptions, SessionResume, resume_session},
    snapshot::{
        DialogueSessionSnapshot, decode_session_messagepack, effect_request_from_snapshot,
        encode_session_messagepack, snapshot_session,
    },
    start_scene,
};
use recite_testkit::{
    Answers, Dialogue, EffectObservation, HistoryItem, ReferenceProgram, Rejection, answers,
    apply_patch, drive, patch, program,
};

use super::support::{MAX_STEPS, RuntimeDialogue, Save, compile, effect, failure, levels};

/// A revision of a generated program: its asset and its source AST.
struct Revision {
    asset: CompiledDialogue,
    reference: ReferenceProgram,
}

impl Revision {
    fn new(program: &recite_testkit::Program) -> Result<Self, TestCaseError> {
        Ok(Self {
            asset: compile(program)?,
            reference: ReferenceProgram::lower(&program.sources()).map_err(failure)?,
        })
    }
}

/// Plays the baseline with the answers' policy, saving at every boundary.
pub(super) fn saves(
    asset: &CompiledDialogue,
    answers: &Answers,
) -> Result<Vec<Save>, TestCaseError> {
    let session = start_scene(asset, None).map_err(failure)?;
    let mut dialogue = RuntimeDialogue::new(asset, session, answers, Vec::new()).saving();
    drive(&mut dialogue, answers, MAX_STEPS);
    prop_assert_eq!(dialogue.failure, None);
    Ok(dialogue.saves.unwrap_or_default())
}

fn saved_effects(
    snapshot: &DialogueSessionSnapshot,
) -> Result<Vec<EffectObservation>, TestCaseError> {
    snapshot
        .deferred_effects
        .iter()
        .map(|saved| effect_request_from_snapshot(saved).map(|request| effect(&request)))
        .collect::<Result<_, _>>()
        .map_err(failure)
}

/// Prompts match when they share their line or any choice, as resume
/// locates a prompt that gained or lost choices.
fn same_history(saved: &[HistoryItem], resumed: &[HistoryItem]) -> bool {
    saved.len() == resumed.len()
        && saved.iter().zip(resumed).all(|pair| match pair {
            (
                HistoryItem::Prompt { line, choices },
                HistoryItem::Prompt {
                    line: other_line,
                    choices: other_choices,
                },
            ) => {
                (line.is_some() && line == other_line)
                    || choices.iter().any(|choice| other_choices.contains(choice))
            }
            (left, right) => left == right,
        })
}

/// Resumes `save`, taken in `baseline`, against `candidate` and checks what
/// every resume must guarantee.
fn check_resume(
    save: &Save,
    baseline: &Revision,
    candidate: &Revision,
    answers: &Answers,
) -> Result<Option<ResumeLevel>, TestCaseError> {
    let saved = &save.snapshot;
    let resumed =
        resume_session(&candidate.asset, saved.clone(), &ResumeOptions::new()).map_err(failure)?;
    let (mut session, report) = match resumed {
        SessionResume::Fallback(fallback) => {
            prop_assert_eq!(fallback.ended, saved.ended);
            if saved.ended {
                // The end already delivered every effect.
                prop_assert!(fallback.deferred_effects.is_empty(), "{fallback:?}");
                prop_assert!(fallback.pending_effect.is_none(), "{fallback:?}");
            } else {
                let deferred: Vec<_> = fallback.deferred_effects.iter().map(effect).collect();
                prop_assert_eq!(deferred, saved_effects(saved)?);
                prop_assert_eq!(
                    fallback.pending_effect.is_some(),
                    saved.pending_effect.is_some()
                );
                let history: Vec<_> = fallback
                    .selected_choice_history
                    .iter()
                    .map(|id| id.as_str().to_owned())
                    .collect();
                prop_assert_eq!(&history, &saved.selected_choice_history);
            }
            return Ok(None);
        }
        SessionResume::Resumed { session, report } => (session, report),
        other => return Err(failure(other)),
    };

    // Saving the resumed session again restores it exactly.
    let resaved = snapshot_session(&candidate.asset, &session).map_err(failure)?;
    let bytes = encode_session_messagepack(&candidate.asset, &session).map_err(failure)?;
    let restored = decode_session_messagepack(&candidate.asset, &bytes).map_err(failure)?;
    prop_assert_eq!(
        snapshot_session(&candidate.asset, &restored).map_err(failure)?,
        resaved.clone()
    );

    if saved.ended {
        let mut dialogue = RuntimeDialogue::new(&candidate.asset, session, answers, Vec::new());
        prop_assert_eq!(dialogue.next(), Err(Rejection::SessionEnded));
        return Ok(Some(report.level));
    }

    // The resumed position follows the same history since the latest line or
    // prompt as the saved one: no unseen line or effect was skipped.
    let saved_history = baseline
        .reference
        .history(&saved.block_id, &levels(&baseline.asset, saved))
        .map_err(failure)?;
    // A prompt whose block changed in structure is evaluated again, so the
    // session resumes at the prompt rather than after it.
    let mut resumed_levels = levels(&candidate.asset, &resaved);
    if report.prompt == PromptResume::Reevaluated
        && let Some(level) = resumed_levels.last_mut()
    {
        level.next += 1;
    }
    let resumed_history = candidate
        .reference
        .history(&resaved.block_id, &resumed_levels)
        .map_err(failure)?;
    prop_assert!(
        same_history(&saved_history, &resumed_history),
        "saved after {saved_history:?}, resumed after {resumed_history:?} ({report:?})"
    );

    // From there, the resumed session plays like the reference interpreter
    // started at the same position with the saved effects.
    let pending = resaved.pending_prompt.is_some() || resaved.pending_effect.is_some();
    let mut oracle = candidate
        .reference
        .resume_at(
            answers,
            &resaved.block_id,
            &levels(&candidate.asset, &resaved),
            pending,
            saved_effects(saved)?,
        )
        .map_err(failure)?;
    let expected = drive(&mut oracle, answers, MAX_STEPS);
    let mut dialogue = RuntimeDialogue::new(&candidate.asset, session.clone(), answers, Vec::new());
    let actual = drive(&mut dialogue, answers, MAX_STEPS);
    prop_assert_eq!(
        actual,
        expected,
        "resumed {:?} in block {}",
        report,
        resaved.block_id
    );

    // A pending effect is acknowledged by its saved request ID.
    if let Some(pending) = &saved.pending_effect {
        let id = effect_request_from_snapshot(&pending.request)
            .map_err(failure)?
            .id;
        let result = acknowledge_effect(&mut session, id, EffectAck::Completed);
        prop_assert!(result.is_ok(), "{result:?}");
    }
    Ok(Some(report.level))
}

proptest! {
    /// Every save resumes exactly in its own asset and continues as the
    /// rest of the run did. In a patched revision a save either falls back
    /// with what it saved, or resumes after the same history and continues
    /// like the reference interpreter from that point.
    #[test]
    fn saves_resume_in_their_own_and_patched_revisions(
        program in program(),
        ops in patch(),
        answers in answers(),
    ) {
        let baseline = Revision::new(&program)?;
        let candidate = Revision::new(&apply_patch(&program, &ops))?;
        for save in saves(&baseline.asset, &answers)? {
            let level = check_resume(&save, &baseline, &baseline, &answers)?;
            prop_assert_eq!(level, Some(ResumeLevel::Exact));
            check_resume(&save, &baseline, &candidate, &answers)?;
        }
    }
}
