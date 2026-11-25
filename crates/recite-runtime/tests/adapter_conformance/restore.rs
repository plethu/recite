//! `load_session`: restores a save against the current asset and reports
//! the restore outcome by the names hosts use (`recite_adapter::resume_names`).

use recite_runtime::resume::{
    ResumeFallbackReason, ResumeLevel, ResumeOptions, SessionResume, resume_session,
};

use super::{
    ActiveSession, CATEGORY_ASSET_LOAD_OR_DECODE_ERROR, CATEGORY_NO_ACTIVE_SESSION_ERROR,
    CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR, CATEGORY_SESSION_ALREADY_ACTIVE_ERROR,
    CATEGORY_VALIDATION_ERROR, ReferenceDriver, StepOutcome, StepResult,
};

impl ReferenceDriver {
    pub(super) fn load_session(
        &mut self,
        snapshot_slot: &str,
        asset_slot: &str,
        max_resume_level: Option<&str>,
    ) -> StepResult {
        if self.active_session.is_some() {
            return self.error(
                CATEGORY_SESSION_ALREADY_ACTIVE_ERROR,
                "load_session requested while a session is already active".to_owned(),
            );
        }

        let Some(snapshot) = self.snapshots.get(snapshot_slot).cloned() else {
            return self.error(
                CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR,
                format!("snapshot slot `{snapshot_slot}` is not available"),
            );
        };
        let Some(asset) = self.imported_assets.get(asset_slot).cloned() else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("imported asset slot `{asset_slot}` is not available"),
            );
        };
        let mut options = ResumeOptions::new();
        if let Some(level) = max_resume_level {
            let Some(level) = resume_level_from_name(level) else {
                return self.error(
                    CATEGORY_VALIDATION_ERROR,
                    format!("unknown resume level `{level}`"),
                );
            };
            options = options.with_max_level(level);
        }

        match resume_session(&asset.dialogue, snapshot, &options) {
            // An ended save has nothing to restore in any revision.
            Ok(SessionResume::Resumed { session, .. }) if session.is_ended() => self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "the save was taken after its conversation ended".to_owned(),
            ),
            Ok(SessionResume::Fallback(fallback)) if fallback.ended => self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "the save was taken after its conversation ended".to_owned(),
            ),
            Ok(SessionResume::Resumed { session, report }) => {
                self.active_session = Some(ActiveSession {
                    asset_slot: asset_slot.to_owned(),
                    session,
                });
                self.last_event = None;
                self.current_prompt_choice_ids.clear();
                StepResult::Ok(StepOutcome {
                    resume_level: Some(resume_level_name(report.level).to_owned()),
                    ..self.outcome_without_event()
                })
            }
            Ok(SessionResume::Fallback(fallback)) => StepResult::Ok(StepOutcome {
                fallback_reason: Some(fallback_reason_name(&fallback.reason).to_owned()),
                deferred_effect_functions: Some(
                    fallback
                        .deferred_effects
                        .iter()
                        .map(|effect| effect.function.clone())
                        .collect(),
                ),
                ..self.empty_outcome()
            }),
            Ok(_) => self.error(
                CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR,
                "unknown resume outcome".to_owned(),
            ),
            Err(error) => {
                let category = self.map_restore_runtime_error(&error);
                self.error(category, error.to_string())
            }
        }
    }
}

fn resume_level_name(level: ResumeLevel) -> &'static str {
    match level {
        ResumeLevel::Exact => "exact",
        ResumeLevel::UnchangedBlock => "unchanged_block",
        ResumeLevel::TextEdited => "text_edited",
        ResumeLevel::Relocated => "relocated",
        _ => "unknown",
    }
}

fn resume_level_from_name(name: &str) -> Option<ResumeLevel> {
    [
        ResumeLevel::Exact,
        ResumeLevel::UnchangedBlock,
        ResumeLevel::TextEdited,
        ResumeLevel::Relocated,
    ]
    .into_iter()
    .find(|level| resume_level_name(*level) == name)
}

fn fallback_reason_name(reason: &ResumeFallbackReason) -> &'static str {
    match reason {
        ResumeFallbackReason::BlockRemoved => "block_removed",
        ResumeFallbackReason::LandmarkRemoved { .. } => "landmark_removed",
        ResumeFallbackReason::LandmarkMoved { .. } => "landmark_moved",
        ResumeFallbackReason::LandmarkAmbiguous { .. } => "landmark_ambiguous",
        ResumeFallbackReason::StatementsChanged { .. } => "statements_changed",
        ResumeFallbackReason::BranchRemoved => "branch_removed",
        ResumeFallbackReason::LevelRefused { .. } => "level_refused",
        _ => "unknown",
    }
}
