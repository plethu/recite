//! Structured, host-independent preview over the deterministic runtime.
//!
//! Preview owns only host interaction and projections. Traversal, condition
//! semantics, effect construction, and session save data remain owned by the
//! runtime modules underneath it.

mod commands;
mod condition;
mod driver;
mod lifecycle;
mod model;
mod projection;
mod snapshot;
mod snapshot_codec;

use recite_core::CompiledDialogue;

use self::condition::PendingOperation;
use crate::DialogueSession;

pub use model::{
    ConditionAnswer, PREVIEW_SNAPSHOT_FORMAT_VERSION, PreviewCommand, PreviewConditionArgument,
    PreviewConditionQuery, PreviewConditionRequest, PreviewConditionRequestId,
    PreviewConditionResult, PreviewError, PreviewEvent, PreviewInputRevision, PreviewInputs,
    PreviewOptions, PreviewOutput, PreviewPrompt, PreviewPromptIdentity, PreviewRestartRequirement,
    PreviewSessionState, PreviewSnapshot, PreviewState, PreviewStatus, PreviewTrace,
    PreviewTranscript, PreviewTranscriptEvent,
};

/// A structured preview over one borrowed compiled dialogue asset.
pub struct PreviewSession<'asset> {
    asset: &'asset CompiledDialogue,
    block: Option<String>,
    options: PreviewOptions,
    session: DialogueSession,
    state: PreviewState,
    trace: PreviewTrace,
    transcript: PreviewTranscript,
    pending: Option<PendingOperation>,
    next_condition_id: PreviewConditionRequestId,
    restored_effect_reemit: Option<recite_core::EffectId>,
}

impl<'asset> PreviewSession<'asset> {
    #[must_use]
    pub fn state(&self) -> &PreviewState {
        &self.state
    }

    #[must_use]
    pub fn session(&self) -> &DialogueSession {
        &self.session
    }

    #[must_use]
    pub fn trace(&self) -> &PreviewTrace {
        &self.trace
    }

    #[must_use]
    pub fn transcript(&self) -> &PreviewTranscript {
        &self.transcript
    }
}
