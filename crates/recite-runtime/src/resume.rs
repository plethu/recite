//! Resume saved sessions, including against patched compiled assets.
//!
//! A snapshot identifies its position twice: by table indices, valid only
//! for the exact compiled payload it was saved against, and by a resume
//! anchor built from authored block, line and choice IDs. When the payload
//! differs, the anchor relocates the position deterministically or the save
//! falls back with a structured reason. Resume never guesses between
//! ambiguous positions.

mod anchor;
mod digest;
mod impact;
mod key;
mod locate;
mod outcome;
mod relocate;

pub(crate) use anchor::{ResumeAnchor, resume_anchor};
pub(crate) use digest::{BlockDigests, DIGEST_ALGORITHM, block_digests};
pub use impact::{
    SaveImpactReport, SaveImpactSummary, SavePointImpact, SavePointKind, SavePointOutcome,
    analyze_save_impact,
};
pub use outcome::{
    PromptResume, ResumeFallback, ResumeFallbackReason, ResumeLandmark, ResumeLevel, ResumeOptions,
    ResumeReport, SessionResume,
};
pub use relocate::{resume_session, resume_session_messagepack};
