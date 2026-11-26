//! Saves taken along a run of a generated program, resumed in the same asset
//! and in a patched revision.

use proptest::prelude::*;
use recite_core::{LocaleId, compiled::CompiledDialogue};
use recite_runtime::{
    DialogueEffectRequest, DialogueEvent, DialogueSessionOptions, EffectAck, acknowledge_effect,
    resume::{
        PromptResume, ResumeFallback, ResumeFallbackReason, ResumeLandmark, ResumeLevel,
        ResumeOptions, SessionResume, resume_session,
    },
    snapshot::{
        DialogueSessionSnapshot, decode_session_messagepack, effect_request_from_snapshot,
        encode_session_messagepack, snapshot_session,
    },
    start_scene_with_options,
};
use recite_testkit::{
    Answers, Dialogue, EffectObservation, HistoryItem, PatchOp, ReferenceProgram, Rejection,
    answers, apply_patch, drive, patch, program,
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
    // A locale, so fallbacks and resumes must carry the saved one.
    let locale = LocaleId::new("fr-FR").map_err(failure)?;
    let options = DialogueSessionOptions::new().with_locale(locale);
    let session = start_scene_with_options(asset, None, options).map_err(failure)?;
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

/// The deferred effects a save holds, exactly as saved.
fn saved_requests(
    snapshot: &DialogueSessionSnapshot,
) -> Result<Vec<DialogueEffectRequest>, TestCaseError> {
    snapshot
        .deferred_effects
        .iter()
        .map(effect_request_from_snapshot)
        .collect::<Result<_, _>>()
        .map_err(failure)
}

/// A landmark as `(line, choices, is_prompt)`, from resume or from the
/// reference history, whose last item names the landmark or the block start.
type LandmarkKey = Option<(Option<String>, Vec<String>, bool)>;

fn history_landmark(history: &[HistoryItem]) -> LandmarkKey {
    match history.last()? {
        HistoryItem::Line(id) => Some((Some(id.clone()), Vec::new(), false)),
        HistoryItem::Prompt { line, choices } => Some((line.clone(), choices.clone(), true)),
        _ => None,
    }
}

fn resume_landmark(landmark: Option<&ResumeLandmark>) -> Result<LandmarkKey, TestCaseError> {
    let ids = |ids: &[recite_core::ChoiceId]| ids.iter().map(|id| id.as_str().to_owned()).collect();
    Ok(match landmark {
        None => None,
        Some(ResumeLandmark::Line(id)) => Some((Some(id.as_str().to_owned()), Vec::new(), false)),
        Some(ResumeLandmark::Prompt { line, choices }) => Some((
            line.as_ref().map(|line| line.as_str().to_owned()),
            ids(choices),
            true,
        )),
        Some(other) => return Err(failure(other)),
    })
}

/// A fallback hands the host exactly what was saved: block, landmark,
/// locale, choice history, deferred effects and pending effect, with their
/// request IDs. An ended save carries no effects, as its end delivered them.
fn check_fallback(
    fallback: &ResumeFallback,
    saved: &DialogueSessionSnapshot,
    baseline: &Revision,
    candidate: &Revision,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(fallback.ended, saved.ended);
    prop_assert_eq!(fallback.block_id.as_str(), saved.block_id.as_str());
    // The reason names a missing block exactly when the block is gone, and
    // default options accept every level, so none is refused.
    let available = candidate
        .asset
        .blocks
        .iter()
        .any(|block| block.id.as_str() == saved.block_id);
    prop_assert_eq!(fallback.block_available, available);
    prop_assert_eq!(
        fallback.reason == ResumeFallbackReason::BlockRemoved,
        !available,
        "{:?}",
        fallback.reason
    );
    prop_assert!(
        !matches!(fallback.reason, ResumeFallbackReason::LevelRefused { .. }),
        "{:?}",
        fallback.reason
    );
    prop_assert_eq!(
        fallback.locale.as_ref().map(ToString::to_string),
        saved.locale.clone()
    );
    let history: Vec<_> = fallback
        .selected_choice_history
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    prop_assert_eq!(&history, &saved.selected_choice_history);
    if saved.ended {
        prop_assert!(fallback.deferred_effects.is_empty(), "{fallback:?}");
        prop_assert!(fallback.pending_effect.is_none(), "{fallback:?}");
        prop_assert!(fallback.landmark.is_none(), "{fallback:?}");
        return Ok(());
    }
    prop_assert_eq!(&fallback.deferred_effects, &saved_requests(saved)?);
    let pending = saved
        .pending_effect
        .as_ref()
        .map(|pending| effect_request_from_snapshot(&pending.request))
        .transpose()
        .map_err(failure)?;
    prop_assert_eq!(&fallback.pending_effect, &pending);
    let saved_history = baseline
        .reference
        .history(&saved.block_id, &levels(&baseline.asset, saved))
        .map_err(failure)?;
    prop_assert_eq!(
        resume_landmark(fallback.landmark.as_ref())?,
        history_landmark(&saved_history)
    );
    Ok(())
}

/// Resumes every ended save outside the default block against a revision
/// without that block, so the ended-save fallback is checked on every run
/// that ends elsewhere: it must carry no effects, as the end delivered them.
fn check_ended_saves_without_their_block(
    program: &recite_testkit::Program,
    saves: &[Save],
    baseline: &Revision,
    answers: &Answers,
) -> Result<(), TestCaseError> {
    for save in saves.iter().filter(|save| save.snapshot.ended) {
        let Some(removed) = program.without_block(&save.snapshot.block_id) else {
            continue;
        };
        let candidate = Revision::new(&removed)?;
        prop_assert_eq!(check_resume(save, baseline, &candidate, answers)?, None);
    }
    Ok(())
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
            check_fallback(&fallback, saved, baseline, candidate)?;
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
    // A waiting prompt keeps its saved availability unless its block
    // changed in structure, when it is evaluated again.
    if saved.pending_prompt.is_some() {
        let expected = if report.level <= ResumeLevel::TextEdited {
            PromptResume::Kept
        } else {
            PromptResume::Reevaluated
        };
        prop_assert_eq!(report.prompt, expected, "at {:?}", report.level);
    }
    let mut dialogue = RuntimeDialogue::new(&candidate.asset, session.clone(), answers, Vec::new());
    if report.prompt == PromptResume::Kept {
        dialogue = dialogue.presenting_saved_prompt();
    }
    let actual = drive(&mut dialogue, answers, MAX_STEPS);
    prop_assert_eq!(
        actual,
        expected,
        "resumed {:?} in block {}",
        report,
        resaved.block_id
    );
    // Saved deferred effects are delivered as saved, request IDs included.
    let ended = dialogue.events.iter().find_map(|event| match event {
        DialogueEvent::End { deferred_effects } => Some(deferred_effects),
        _ => None,
    });
    if let Some(delivered) = ended {
        let saved_deferred = saved_requests(saved)?;
        prop_assert!(
            delivered.starts_with(&saved_deferred),
            "delivered {delivered:?}, saved {saved_deferred:?}"
        );
    }

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
        let patched = apply_patch(&program, &ops);
        let baseline = Revision::new(&program)?;
        let candidate = Revision::new(&patched)?;
        let text_only = ops
            .iter()
            .all(|op| matches!(op, PatchOp::EditLineText(_) | PatchOp::EditChoiceText(_)));
        let saves = saves(&baseline.asset, &answers)?;
        check_ended_saves_without_their_block(&program, &saves, &baseline, &answers)?;
        for save in saves {
            let level = check_resume(&save, &baseline, &baseline, &answers)?;
            prop_assert_eq!(level, Some(ResumeLevel::Exact));
            let level = check_resume(&save, &baseline, &candidate, &answers)?;
            if save.snapshot.ended {
                continue;
            }
            // A save must resume where the specification says it does: in an
            // unchanged block, and after a patch that only edits text.
            let block = save.snapshot.block_id.as_str();
            if program.block_source(block).is_some()
                && program.block_source(block) == patched.block_source(block)
            {
                prop_assert!(
                    matches!(level, Some(ResumeLevel::Exact | ResumeLevel::UnchangedBlock)),
                    "a save in unchanged block `{block}` resumed at {level:?}"
                );
            } else if text_only {
                prop_assert!(
                    level.is_some_and(|level| level <= ResumeLevel::TextEdited),
                    "a save in text-edited block `{block}` resumed at {level:?}"
                );
            }
        }
    }
}
