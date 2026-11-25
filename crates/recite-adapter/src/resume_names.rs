//! Stable machine names for restore outcomes, shared by engine projections.
//!
//! Engines that cannot carry Rust enums (Godot dictionaries, the C ABI's
//! MessagePack report, Unity) use these names so every host reports the same
//! outcome the same way.

use recite_runtime::resume::{PromptResume, ResumeFallbackReason, ResumeLandmark, ResumeLevel};

#[must_use]
pub fn resume_level_name(level: ResumeLevel) -> &'static str {
    match level {
        ResumeLevel::Exact => "exact",
        ResumeLevel::UnchangedBlock => "unchanged_block",
        ResumeLevel::TextEdited => "text_edited",
        ResumeLevel::Relocated => "relocated",
        _ => "unknown",
    }
}

/// Parses a host-supplied maximum level; unknown names are rejected.
#[must_use]
pub fn parse_resume_level(name: &str) -> Option<ResumeLevel> {
    Some(match name {
        "exact" => ResumeLevel::Exact,
        "unchanged_block" => ResumeLevel::UnchangedBlock,
        "text_edited" => ResumeLevel::TextEdited,
        "relocated" => ResumeLevel::Relocated,
        _ => return None,
    })
}

#[must_use]
pub fn prompt_resume_name(prompt: PromptResume) -> &'static str {
    match prompt {
        PromptResume::Kept => "kept",
        PromptResume::Reevaluated => "reevaluated",
        PromptResume::None => "none",
        _ => "unknown",
    }
}

#[must_use]
pub fn fallback_reason_name(reason: &ResumeFallbackReason) -> &'static str {
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

/// The landmark a fallback reason names, if any.
#[must_use]
pub fn fallback_reason_landmark(reason: &ResumeFallbackReason) -> Option<&ResumeLandmark> {
    match reason {
        ResumeFallbackReason::LandmarkRemoved { landmark }
        | ResumeFallbackReason::LandmarkMoved { landmark }
        | ResumeFallbackReason::LandmarkAmbiguous { landmark } => Some(landmark),
        ResumeFallbackReason::StatementsChanged { landmark } => landmark.as_ref(),
        _ => None,
    }
}

/// The landmark's line ID, if it has one.
#[must_use]
pub fn landmark_line(landmark: &ResumeLandmark) -> Option<&str> {
    match landmark {
        ResumeLandmark::Line(line) => Some(line.as_str()),
        ResumeLandmark::Prompt { line, .. } => line.as_ref().map(|line| line.as_str()),
        _ => None,
    }
}

/// The landmark's choice IDs; empty for a line.
#[must_use]
pub fn landmark_choices(landmark: &ResumeLandmark) -> Vec<&str> {
    match landmark {
        ResumeLandmark::Prompt { choices, .. } => {
            choices.iter().map(|choice| choice.as_str()).collect()
        }
        _ => Vec::new(),
    }
}

#[must_use]
pub fn landmark_kind(landmark: &ResumeLandmark) -> &'static str {
    match landmark {
        ResumeLandmark::Line(_) => "line",
        ResumeLandmark::Prompt { .. } => "prompt",
        _ => "unknown",
    }
}
