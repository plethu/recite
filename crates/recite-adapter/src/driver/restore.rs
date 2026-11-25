//! Restore saved sessions, including against patched compiled assets.

use recite_runtime::{
    DialogueContext, DialogueEvent, LocaleResolution,
    resume::{
        ResumeFallback, ResumeOptions, ResumeReport, SessionResume, resume_session_messagepack,
    },
};

use super::{ActiveSession, DriverError, SessionDriver, SessionState, collapse, no_active};
use crate::{AdapterError, AdapterErrorKind, AdapterResult, LoadedDialogue};

/// What restoring a save produced.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum RestoreOutcome<T> {
    /// The saved session is active again. `output` is the first batch after
    /// restore, including a re-presented prompt or re-emitted blocking effect.
    Resumed { report: ResumeReport, output: T },
    /// The save could not resume in this asset; no session is active. The
    /// fallback carries the saved block, deferred and pending effects.
    Fallback(Box<ResumeFallback>),
}

/// The result of preparing a restore without traversal.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreparedRestore {
    /// A session is prepared; call `begin` after configuring the host.
    Resumed(ResumeReport),
    /// No session was prepared.
    Fallback(Box<ResumeFallback>),
}

impl SessionDriver {
    /// Restores a save against `asset`, which may be a patched revision of the
    /// asset it was saved against. `options` bounds how far the save may move.
    pub fn restore(
        &mut self,
        asset: &LoadedDialogue,
        bytes: &[u8],
        options: &ResumeOptions,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<RestoreOutcome<Vec<DialogueEvent>>> {
        collapse(self.restore_with(
            asset,
            bytes,
            options,
            context,
            resolution,
            Ok::<_, std::convert::Infallible>,
        ))
    }

    pub fn restore_with<T, E>(
        &mut self,
        asset: &LoadedDialogue,
        bytes: &[u8],
        options: &ResumeOptions,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<RestoreOutcome<T>, DriverError<E>> {
        let report = match self.prepare_restore(asset, bytes, options)? {
            PreparedRestore::Resumed(report) => report,
            PreparedRestore::Fallback(fallback) => return Ok(RestoreOutcome::Fallback(fallback)),
        };
        let result = self.begin_with(context, resolution, encode);
        match result {
            Ok(output) => Ok(RestoreOutcome::Resumed { report, output }),
            Err(error) => {
                self.active = None;
                self.observed_choices.clear();
                Err(error)
            }
        }
    }

    /// Restores the checkpoint without traversing. Configure host conditions
    /// and locale resolution before calling `begin`, as for a new session.
    /// Unsupported or malformed saves, and saves of a different asset, are
    /// errors; a save that cannot resume in this asset is a fallback.
    pub fn prepare_restore(
        &mut self,
        asset: &LoadedDialogue,
        bytes: &[u8],
        options: &ResumeOptions,
    ) -> AdapterResult<PreparedRestore> {
        if self.has_active_session() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }
        let (session, report) = match resume_session_messagepack(asset.dialogue(), bytes, options)
            .map_err(AdapterError::from_restore_error)?
        {
            SessionResume::Resumed { session, report } => (session, report),
            SessionResume::Fallback(fallback) => {
                return Ok(PreparedRestore::Fallback(Box::new(fallback)));
            }
            _ => {
                return Err(AdapterError::with_detail(
                    AdapterErrorKind::SaveLoadIncompatibility,
                    "unsupported resume outcome",
                ));
            }
        };
        if session.is_ended() {
            return Err(no_active());
        }
        self.observed_choices = session
            .previous_prompt_choices()
            .iter()
            .chain(session.selected_choice_history())
            .cloned()
            .collect();
        self.active = Some(SessionState::Prepared(ActiveSession {
            asset: asset.clone(),
            session,
        }));
        Ok(PreparedRestore::Resumed(report))
    }
}
