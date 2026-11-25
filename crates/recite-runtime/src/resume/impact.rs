//! Classify every save point of a shipped asset against a patched one.
//!
//! The analysis saves a session at each reachable host-observable boundary of the
//! baseline and resumes it against the candidate through
//! [`resume_session`](crate::resume::resume_session),
//! so tooling reports exactly what players' saves will do.

use recite_core::{
    BlockId, ChoiceId, LineId, SourceSpan,
    compiled::{BlockIndex, CompiledDialogue},
};

use crate::DialogueError;
use crate::traversal::AssetView;

use super::outcome::{PromptResume, ResumeFallbackReason, ResumeLevel};

mod walk;
use walk::Walker;

/// Every save point in a baseline asset, classified against a candidate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct SaveImpactReport {
    /// The candidate is byte-for-byte the baseline payload.
    pub identical: bool,
    pub schema_changed: bool,
    pub compiler_changed: bool,
    /// Save points in baseline block-table order, then source order.
    pub points: Vec<SavePointImpact>,
}

/// Counts of save points by outcome.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SaveImpactSummary {
    pub total: usize,
    pub exact: usize,
    pub unchanged_block: usize,
    pub text_edited: usize,
    pub relocated: usize,
    /// Resumed points whose prompt is evaluated again from patched content.
    pub reevaluated_prompts: usize,
    pub fallback: usize,
}

impl SaveImpactReport {
    #[must_use]
    pub fn summary(&self) -> SaveImpactSummary {
        let mut summary = SaveImpactSummary {
            total: self.points.len(),
            ..SaveImpactSummary::default()
        };
        for point in &self.points {
            match &point.outcome {
                SavePointOutcome::Resumes { level, prompt } => {
                    match level {
                        ResumeLevel::Exact => summary.exact += 1,
                        ResumeLevel::UnchangedBlock => summary.unchanged_block += 1,
                        ResumeLevel::TextEdited => summary.text_edited += 1,
                        ResumeLevel::Relocated => summary.relocated += 1,
                    }
                    if *prompt == PromptResume::Reevaluated {
                        summary.reevaluated_prompts += 1;
                    }
                }
                SavePointOutcome::Fallback { .. } => summary.fallback += 1,
            }
        }
        summary
    }

    pub fn fallbacks(&self) -> impl Iterator<Item = &SavePointImpact> {
        self.points
            .iter()
            .filter(|point| matches!(point.outcome, SavePointOutcome::Fallback { .. }))
    }
}

/// One place a player can save, and what that save does after the patch.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct SavePointImpact {
    pub block_id: BlockId,
    pub kind: SavePointKind,
    /// Where the save point is in the baseline source.
    pub baseline_span: SourceSpan,
    /// The saved block's header in the candidate source, when it still exists.
    pub candidate_block_span: Option<SourceSpan>,
    pub outcome: SavePointOutcome,
}

/// The host-observable boundary a save was taken at.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SavePointKind {
    BlockStart,
    AfterLine {
        line: LineId,
    },
    Prompt {
        line: Option<LineId>,
        choices: Vec<ChoiceId>,
    },
    BlockingEffect {
        function: String,
    },
    AfterImmediateEffect {
        function: String,
    },
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SavePointOutcome {
    Resumes {
        level: ResumeLevel,
        prompt: PromptResume,
    },
    Fallback {
        reason: ResumeFallbackReason,
        block_available: bool,
    },
}

/// Classifies every save point of `baseline` against `candidate`.
///
/// Both assets must share an asset ID. The result does not depend on game
/// state: prompts are saved with every choice available, and saved
/// availability does not affect where a save resumes.
pub fn analyze_save_impact(
    baseline: &CompiledDialogue,
    candidate: &CompiledDialogue,
) -> Result<SaveImpactReport, DialogueError> {
    let baseline_view = AssetView::new(baseline)?;
    let candidate_view = AssetView::new(candidate)?;
    let payload = baseline
        .content_fingerprint()
        .map_err(|error| DialogueError::MalformedCompiledAsset {
            reason: error.to_string(),
        })?
        .clone();
    let identical = candidate.content_fingerprint().ok() == Some(&payload);
    let mut walker = Walker::new(baseline, baseline_view, candidate, candidate_view, payload);
    for index in 0..baseline.blocks.len() {
        walker.block(BlockIndex::new(u32::try_from(index).unwrap_or(u32::MAX)))?;
    }
    Ok(SaveImpactReport {
        identical,
        schema_changed: baseline.header.schema_fingerprint != candidate.header.schema_fingerprint,
        compiler_changed: baseline.header.compiler_version != candidate.header.compiler_version,
        points: walker.into_points(),
    })
}
