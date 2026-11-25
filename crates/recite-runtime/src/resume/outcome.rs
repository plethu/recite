//! Structured results of resuming a saved session against a compiled asset.

use recite_core::{BlockId, ChoiceId, LineId, LocaleId, compiled::CompiledDialogue};

use crate::{
    DialogueEffectRequest, DialogueError, DialogueSession, DialogueSessionOptions,
    start_scene_with_options,
};

/// How far a resumed save moved from the asset it was saved against, in
/// increasing order of change.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResumeLevel {
    /// The asset is the one the session was saved against.
    Exact,
    /// The asset changed, but not the block the session was saved in.
    UnchangedBlock,
    /// Only text, speakers, interpolation or metadata changed in the saved
    /// block; the session resumes at the same point and shows the new text.
    TextEdited,
    /// The saved block's structure changed, and the saved position was found
    /// again through its stable IDs and the statements around it.
    Relocated,
}

/// Restore options chosen by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeOptions {
    max_level: ResumeLevel,
}

impl Default for ResumeOptions {
    fn default() -> Self {
        Self {
            max_level: ResumeLevel::Relocated,
        }
    }
}

impl ResumeOptions {
    /// Accepts any deterministic relocation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resumes only against the exact asset; any change falls back.
    #[must_use]
    pub fn exact_only() -> Self {
        Self {
            max_level: ResumeLevel::Exact,
        }
    }

    /// The most change a resumed session may absorb. A save that needs more
    /// becomes a [`ResumeFallback`] with [`ResumeFallbackReason::LevelRefused`].
    #[must_use]
    pub fn with_max_level(mut self, max_level: ResumeLevel) -> Self {
        self.max_level = max_level;
        self
    }

    #[must_use]
    pub fn max_level(&self) -> ResumeLevel {
        self.max_level
    }
}

/// The result of resuming a saved session.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum SessionResume {
    /// The session continues from its saved point.
    Resumed {
        session: DialogueSession,
        report: ResumeReport,
    },
    /// The saved point cannot be resumed in this asset. No session is live;
    /// the fallback carries everything the host needs to continue.
    Fallback(ResumeFallback),
}

/// What changed between the saved asset and the one a session resumed in.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeReport {
    pub level: ResumeLevel,
    pub block_id: BlockId,
    pub prompt: PromptResume,
    /// The schema fingerprint differs from the saved asset's.
    pub schema_changed: bool,
    /// The compiler version differs from the saved asset's.
    pub compiler_changed: bool,
}

/// What happened to a prompt that was waiting when the session was saved.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptResume {
    /// No prompt was waiting.
    None,
    /// The prompt keeps its saved choices and availability. It is presented
    /// again on the next advance.
    Kept,
    /// The patch changed the prompt's choices. The prompt is evaluated again
    /// from the patched content on the next advance, querying conditions.
    Reevaluated,
}

/// A save that could not be resumed in the provided asset.
///
/// Deferred effects and a pending blocking effect are returned exactly as
/// they were saved, so the game can honour what the dialogue already
/// requested before it restarts or ends the conversation. A save taken after
/// the conversation ended is marked [`ResumeFallback::ended`] and returns no
/// effects: the session already delivered them with its end.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct ResumeFallback {
    pub reason: ResumeFallbackReason,
    /// The block the session was saved in.
    pub block_id: BlockId,
    /// Whether the provided asset still contains that block.
    pub block_available: bool,
    /// The last line or prompt before the saved point, if any.
    pub landmark: Option<ResumeLandmark>,
    /// Whether the conversation had already ended when it was saved. There is
    /// nothing to apply, restart or reconcile.
    pub ended: bool,
    pub deferred_effects: Vec<DialogueEffectRequest>,
    pub pending_effect: Option<DialogueEffectRequest>,
    pub selected_choice_history: Vec<ChoiceId>,
    pub locale: Option<LocaleId>,
}

impl ResumeFallback {
    /// Starts a new session at the start of the saved block, in the saved
    /// locale. Deferred effects are not carried over: the host already holds
    /// them in this fallback, and replaying the block would collect them again.
    pub fn restart_block(
        &self,
        asset: &CompiledDialogue,
    ) -> Result<DialogueSession, DialogueError> {
        let mut options = DialogueSessionOptions::new();
        if let Some(locale) = &self.locale {
            options = options.with_locale(locale.clone());
        }
        start_scene_with_options(asset, Some(self.block_id.as_str()), options)
    }
}

/// Why a save fell back.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeFallbackReason {
    /// The saved block no longer exists.
    BlockRemoved,
    /// A line or prompt that located the saved point no longer exists.
    LandmarkRemoved { landmark: ResumeLandmark },
    /// A line or prompt that located the saved point moved to another block
    /// or branch.
    LandmarkMoved { landmark: ResumeLandmark },
    /// The saved prompt's choices now belong to more than one prompt.
    LandmarkAmbiguous { landmark: ResumeLandmark },
    /// Statements between the locating line or prompt and the saved point
    /// changed: a line, effect, condition, divert or prompt was added, removed
    /// or edited there.
    StatementsChanged { landmark: Option<ResumeLandmark> },
    /// The `if` or `match` branch the session was inside no longer exists.
    BranchRemoved,
    /// The saved point needs more change than the host's options allow.
    LevelRefused {
        required: ResumeLevel,
        allowed: ResumeLevel,
    },
}

/// A statement with authored IDs that locates a saved point.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeLandmark {
    Line(LineId),
    Prompt {
        line: Option<LineId>,
        choices: Vec<ChoiceId>,
    },
}
